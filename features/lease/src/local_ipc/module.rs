//! 服务和资源回调可能比调用方的加载引用活得更久；取得一个保留到进程退出的加载引用。
use crate::LeaseError;
use std::{ffi::c_void, mem::MaybeUninit};

pub(super) fn pin() -> Result<(), LeaseError> {
    static PINNED: std::sync::OnceLock<Result<(), LeaseError>> = std::sync::OnceLock::new();
    PINNED.get_or_init(retain).clone()
}

fn retain() -> Result<(), LeaseError> {
    if in_main_executable(pin as *const () as *const c_void) {
        return Ok(());
    }
    let mut info = MaybeUninit::<libc::Dl_info>::uninit();
    // SAFETY: 当前函数地址属于存活模块，info 是有效可写输出。
    if unsafe { libc::dladdr(pin as *const () as *const c_void, info.as_mut_ptr()) } == 0 {
        return Err(LeaseError::LocalIpc("无法定位资源服务模块".into()));
    }
    // SAFETY: dladdr 成功后 info 及加载器持有的路径均有效。
    let info = unsafe { info.assume_init() };
    if info.dli_fname.is_null() {
        return Err(LeaseError::LocalIpc("资源服务模块缺少路径".into()));
    }
    // SAFETY: NOLOAD 只保留现有映射，不从替换后的磁盘文件加载新模块。
    let handle = unsafe { libc::dlopen(info.dli_fname, libc::RTLD_LAZY | libc::RTLD_NOLOAD) };
    if handle.is_null() {
        return Err(LeaseError::LocalIpc("无法固定资源服务模块".into()));
    }
    // 与 Windows PIN 相同：服务回调所在模块保留到进程退出，避免线程返回时卸载自身。
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn in_main_executable(address: *const c_void) -> bool {
    struct Query {
        address: usize,
        found: bool,
    }
    unsafe extern "C" fn inspect(
        info: *mut libc::dl_phdr_info,
        size: usize,
        data: *mut c_void,
    ) -> i32 {
        let prefix = std::mem::offset_of!(libc::dl_phdr_info, dlpi_phnum)
            + std::mem::size_of::<libc::Elf64_Half>();
        if info.is_null() || size < prefix {
            return 0;
        }
        // SAFETY: 仅读取加载器实际提供的结构前缀，不构造较新 libc 完整结构的引用。
        let (name, base, headers, count) = unsafe {
            (
                std::ptr::addr_of!((*info).dlpi_name).read(),
                std::ptr::addr_of!((*info).dlpi_addr).read() as usize,
                std::ptr::addr_of!((*info).dlpi_phdr).read(),
                std::ptr::addr_of!((*info).dlpi_phnum).read() as usize,
            )
        };
        // SAFETY: 加载器的路径字符串在本回调内有效。
        if name.is_null() || unsafe { *name } != 0 {
            return 0;
        }
        if headers.is_null() || count == 0 {
            return 1;
        }
        // SAFETY: data 是本次同步迭代的 Query；headers 含 count 个加载器提供的记录。
        let query = unsafe { &mut *data.cast::<Query>() };
        let headers = unsafe { std::slice::from_raw_parts(headers, count) };
        query.found = headers.iter().any(|header| {
            header.p_type == libc::PT_LOAD
                && base
                    .checked_add(header.p_vaddr as usize)
                    .and_then(|start| {
                        start
                            .checked_add(header.p_memsz as usize)
                            .map(|end| query.address >= start && query.address < end)
                    })
                    .unwrap_or(false)
        });
        1
    }
    let mut query = Query {
        address: address as usize,
        found: false,
    };
    // SAFETY: 回调不展开异常，Query 和加载器记录在同步迭代期间存活。
    unsafe {
        libc::dl_iterate_phdr(Some(inspect), (&mut query as *mut Query).cast());
    }
    query.found
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn in_main_executable(_address: *const c_void) -> bool {
    false
}
