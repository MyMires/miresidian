//! 从本模块代码地址定位已加载共享对象，以可配对释放的引用保护执行器线程。
use crate::{MiresidianError, MiresidianResult};
use std::{ffi::c_void, ptr::NonNull};

pub(super) fn retain() -> MiresidianResult<Option<NonNull<c_void>>> {
    let mut info = std::mem::MaybeUninit::<libc::Dl_info>::uninit();
    // SAFETY: 函数地址属于当前仍由宿主持有的模块；info 指向足够大且可写的输出空间。
    if unsafe { libc::dladdr(retain as *const () as *const c_void, info.as_mut_ptr()) } == 0 {
        return Err(MiresidianError::Internal("无法定位执行器模块".into()));
    }
    // SAFETY: 上一步成功的 dladdr 已初始化输出；路径内存由加载器持有，模块引用保护其有效期。
    let info = unsafe { info.assume_init() };
    if info.dli_fname.is_null() {
        return Err(MiresidianError::Internal("执行器模块缺少路径".into()));
    }

    // 按已映射地址识别主程序，不依赖可能被替换的磁盘文件名。
    // 已加载 .so 的路径被替换时，也不能错误跳过模块引用保护。
    if is_main_address(retain as *const () as *const c_void) {
        return Ok(None);
    }
    // SAFETY: NOLOAD 只为既有对象增加引用，不创建新映射或执行新构造函数。
    // 宿主引用在取得期间保护模块及路径；返回的这一份引用由 ModuleGuard::drop 配对释放。
    let handle = unsafe { libc::dlopen(info.dli_fname, libc::RTLD_LAZY | libc::RTLD_NOLOAD) };
    NonNull::new(handle)
        .map(Some)
        .ok_or_else(|| MiresidianError::Internal("无法持有执行器动态库加载引用".into()))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn is_main_address(address: *const c_void) -> bool {
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
        // SAFETY: 已检查加载器传入的结构大小至少覆盖所读字段；按字段读取而不创建完整结构引用，
        // 兼容仅提供较短 dl_phdr_info 的旧加载器。字段只在本次回调期间使用。
        let (name, base, headers, count) = unsafe {
            (
                std::ptr::addr_of!((*info).dlpi_name).read(),
                std::ptr::addr_of!((*info).dlpi_addr).read() as usize,
                std::ptr::addr_of!((*info).dlpi_phdr).read(),
                std::ptr::addr_of!((*info).dlpi_phnum).read() as usize,
            )
        };
        // SAFETY: 短路检查保证非空后才读取首字节；加载器提供的名称在回调期间有效。
        if name.is_null() || unsafe { *name } != 0 {
            return 0;
        }
        if headers.is_null() || count == 0 {
            return 1;
        }
        // SAFETY: data 来自下方仍存活的独占 Query；headers 是加载器提供的 count 项程序头数组。
        // 已排除空指针和零长度，数组与用户数据均只在同步回调期间借用。
        let query = unsafe { &mut *data.cast::<Query>() };
        // SAFETY: headers/count 已由加载器提供并通过非空检查，切片仅在本次回调内读取。
        let headers = unsafe { std::slice::from_raw_parts(headers, count) };
        query.found = headers.iter().any(|header| {
            if header.p_type != libc::PT_LOAD {
                return false;
            }
            base.checked_add(header.p_vaddr as usize)
                .and_then(|start| {
                    start
                        .checked_add(header.p_memsz as usize)
                        .map(|end| (start, end))
                })
                .is_some_and(|(start, end)| query.address >= start && query.address < end)
        });
        1
    }
    let mut query = Query {
        address: address as usize,
        found: false,
    };
    // SAFETY: 回调不展开 panic；query 存活到同步迭代结束。回调只读取加载器持有的程序头，
    // 不保存其指针到回调之外，也不释放或改变模块加载引用。
    unsafe {
        libc::dl_iterate_phdr(Some(inspect), (&mut query as *mut Query).cast());
    }
    query.found
}
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn is_main_address(_address: *const c_void) -> bool {
    false
}
