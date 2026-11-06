//! 执行器独占持有的模块加载引用，在工作线程全部回收后释放。
//!
//! 平台实现只增加既有映射的引用，不加载另一个副本，也不使用不可逆的 PIN/NODELETE。
//! 宿主在每次 DLL 调用（包括关闭）期间仍须保留自己的加载引用；框架引用仅保护内部线程。
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
use crate::MiresidianResult;
use std::{ffi::c_void, ptr::NonNull};

pub(super) struct ModuleGuard(Option<NonNull<c_void>>);
// SAFETY: 句柄是不在 Rust 中解引用的不透明加载引用；两种平台 API 均允许跨线程取得和释放。
unsafe impl Send for ModuleGuard {}
// SAFETY: 共享访问不能修改或释放该独占引用，释放仅发生在守卫的 Drop 中。
unsafe impl Sync for ModuleGuard {}
impl ModuleGuard {
    pub(super) fn retain() -> MiresidianResult<Self> {
        #[cfg(unix)]
        {
            unix::retain().map(Self)
        }
        #[cfg(windows)]
        {
            windows::retain().map(Self)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(crate::MiresidianError::NotSupported(
                "此平台没有模块加载引用实现".into(),
            ))
        }
    }
}
impl Drop for ModuleGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            // SAFETY: 守卫独占这一份加载引用；Executor 的 Runtime 字段先析构并回收线程。
            // 宿主仍持有当前调用的模块引用，因此释放额外引用不会使正在执行的代码失效。
            #[cfg(unix)]
            unsafe {
                libc::dlclose(handle.as_ptr());
            }
            #[cfg(windows)]
            unsafe {
                windows::release(handle);
            }
        }
    }
}
