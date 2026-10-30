//! Retain by address, with a balanced reference rather than irreversible PIN.
use crate::{MiresidianError, MiresidianResult};
use std::{ffi::c_void, ptr::NonNull};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleExW(flags: u32, address: *const u16, module: *mut *mut c_void) -> i32;
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
}
const FROM_ADDRESS: u32 = 4;
const UNCHANGED_REFCOUNT: u32 = 2;
pub(super) fn retain() -> MiresidianResult<Option<NonNull<c_void>>> {
    let address = retain as *const () as *const u16;
    let mut handle = std::ptr::null_mut();
    // SAFETY: FROM_ADDRESS interprets this module's valid code address, with
    // writable output storage. The host keeps a loader reference during calls.
    if unsafe { GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT, address, &mut handle) } == 0 {
        return Err(MiresidianError::Internal("无法定位执行器模块".into()));
    }
    // SAFETY: null queries the process executable and never increments a ref.
    let main = unsafe { GetModuleHandleW(std::ptr::null()) };
    if main.is_null() {
        return Err(MiresidianError::Internal("无法定位进程主模块".into()));
    }
    if handle == main {
        return Ok(None);
    }
    // SAFETY: host reference keeps the previously resolved module mapped. This
    // second call increments exactly one reference; no PIN flag is used.
    if unsafe { GetModuleHandleExW(FROM_ADDRESS, address, &mut handle) } == 0 {
        return Err(MiresidianError::Internal(
            "无法持有执行器模块加载引用".into(),
        ));
    }
    NonNull::new(handle)
        .map(Some)
        .ok_or_else(|| MiresidianError::Internal("执行器模块引用为空".into()))
}
/// # Safety
/// handle must be the guard's owned reference, not a borrowed module handle.
pub(super) unsafe fn release(handle: NonNull<c_void>) {
    // SAFETY: the guard owns the reference, all its threads have joined, and
    // the host still owns its active calling reference.
    unsafe {
        FreeLibrary(handle.as_ptr());
    }
}
