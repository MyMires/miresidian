//! Compatibility containers share runtime identity under a short lock.
use crate::MiresidianResult;
use std::sync::{Mutex, OnceLock};
/// A compatibility source for generated and existing Rust FFI containers.
#[doc(hidden)]
pub trait RuntimeSource<N: crate::DeviceNode>: Sync {
    fn runtime(
        &self,
        platforms: impl FnOnce() -> crate::PlatformRegistry,
    ) -> MiresidianResult<crate::NodeRuntime<N>>;
}
impl<N: crate::DeviceNode> RuntimeSource<N> for OnceLock<crate::NodeRuntime<N>> {
    fn runtime(
        &self,
        platforms: impl FnOnce() -> crate::PlatformRegistry,
    ) -> MiresidianResult<crate::NodeRuntime<N>> {
        Ok(self
            .get_or_init(|| crate::NodeRuntime::with_platforms(platforms()))
            .clone())
    }
}
impl<N: crate::DeviceNode> RuntimeSource<N> for OnceLock<Mutex<crate::NodeRuntime<N>>> {
    fn runtime(
        &self,
        platforms: impl FnOnce() -> crate::PlatformRegistry,
    ) -> MiresidianResult<crate::NodeRuntime<N>> {
        let mutex =
            self.get_or_init(|| Mutex::new(crate::NodeRuntime::with_platforms(platforms())));
        let runtime = match mutex.lock() {
            Ok(r) => r,
            Err(e) => {
                let r = e.into_inner();
                let _ = r.shutdown();
                mutex.clear_poison();
                r
            }
        };
        Ok(runtime.clone())
    }
}
