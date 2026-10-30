//! Loader reference owned by an executor, released only after threads join.
//!
//! Supported implementations retain existing mappings, never load another copy
//! or use irreversible PIN/NODELETE flags. The caller must keep its host loader
//! reference during every DLL call, including shutdown.
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
use crate::MiresidianResult;
use std::{ffi::c_void, ptr::NonNull};

pub(super) struct ModuleGuard(Option<NonNull<c_void>>);
// SAFETY: this is an opaque loader reference, never dereferenced in Rust.
// Both supported loader APIs allow reference acquisition/release across threads.
unsafe impl Send for ModuleGuard {}
// SAFETY: shared access cannot mutate or release this uniquely owned reference.
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
            // SAFETY: this guard owns one loader reference. The executor field is
            // dropped first, joining all workers. The host retains the calling module.
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
