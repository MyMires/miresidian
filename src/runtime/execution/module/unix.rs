//! Resolve a local code address and retain its already mapped shared object.
use crate::{MiresidianError, MiresidianResult};
use std::{ffi::c_void, ptr::NonNull};

pub(super) fn retain() -> MiresidianResult<Option<NonNull<c_void>>> {
    let mut info = std::mem::MaybeUninit::<libc::Dl_info>::uninit();
    // SAFETY: the function address belongs to this module; info is writable.
    if unsafe { libc::dladdr(retain as *const () as *const c_void, info.as_mut_ptr()) } == 0 {
        return Err(MiresidianError::Internal("无法定位执行器模块".into()));
    }
    // SAFETY: successful dladdr initializes the output and loader-owned path.
    let info = unsafe { info.assume_init() };
    if info.dli_fname.is_null() {
        return Err(MiresidianError::Internal("执行器模块缺少路径".into()));
    }

    // Identify the main executable by mapped address, not mutable filesystem
    // names: replacing a loaded .so pathname must never bypass module retention.
    if is_main_address(retain as *const () as *const c_void) {
        return Ok(None);
    }
    // SAFETY: NOLOAD obtains a reference to an existing object and does not
    // create mappings or run new constructors. The caller's load reference
    // protects this module throughout acquisition; Drop balances this one ref.
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
        // SAFETY: dl_iterate_phdr supplies the original prefix of dl_phdr_info
        // for this callback. Read fields individually instead of making a full
        // struct reference, so older loaders with shorter structs remain valid.
        let (name, base, headers, count) = unsafe {
            (
                std::ptr::addr_of!((*info).dlpi_name).read(),
                std::ptr::addr_of!((*info).dlpi_addr).read() as usize,
                std::ptr::addr_of!((*info).dlpi_phdr).read(),
                std::ptr::addr_of!((*info).dlpi_phnum).read() as usize,
            )
        };
        // SAFETY: the loader's non-null dlpi_name is valid throughout callback.
        if name.is_null() || unsafe { *name } != 0 {
            return 0;
        }
        if headers.is_null() || count == 0 {
            return 1;
        }
        // SAFETY: data points to our live Query and headers is the loader's
        // count-element program-header array, valid until callback returns.
        let query = unsafe { &mut *data.cast::<Query>() };
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
    // SAFETY: callback never unwinds and userdata lives for this synchronous
    // iteration. It only reads loader-owned program headers during callback.
    unsafe {
        libc::dl_iterate_phdr(Some(inspect), (&mut query as *mut Query).cast());
    }
    query.found
}
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn is_main_address(_address: *const c_void) -> bool {
    false
}
