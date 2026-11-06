//! 通过模块内地址取得可配对释放的 Windows 加载引用，不使用不可逆 PIN。
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
    // SAFETY: FROM_ADDRESS 将参数解释为本模块有效代码地址，handle 是可写输出空间。
    // 宿主在整个调用期间持有加载引用；此处仅查询，不增加引用计数。
    if unsafe { GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT, address, &mut handle) } == 0 {
        return Err(MiresidianError::Internal("无法定位执行器模块".into()));
    }
    // SAFETY: NULL 查询进程主模块，不访问字符串且不增加引用计数。
    let main = unsafe { GetModuleHandleW(std::ptr::null()) };
    if main.is_null() {
        return Err(MiresidianError::Internal("无法定位进程主模块".into()));
    }
    if handle == main {
        return Ok(None);
    }
    // SAFETY: 宿主引用保证前一步定位的模块仍映射；本次调用恰好增加一份加载引用。
    // 不使用 PIN，返回引用由守卫在工作线程结束后配对释放。
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
/// handle 必须是守卫独占的额外加载引用，不能传入不拥有的查询句柄。
/// 工作线程须已全部回收，调用方仍须持有保护当前调用的宿主引用。
pub(super) unsafe fn release(handle: NonNull<c_void>) {
    // SAFETY: 调用前提保证守卫拥有该引用、线程已回收，宿主引用仍保护当前执行的代码。
    unsafe {
        FreeLibrary(handle.as_ptr());
    }
}
