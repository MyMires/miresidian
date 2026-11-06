//! 兼容新旧 FFI 容器；旧 Mutex 只保护句柄读取，不覆盖完整业务调用。
use crate::MiresidianResult;
use std::sync::{Mutex, OnceLock};
/// 为宏生成代码和旧 Rust FFI 容器取得共用运行时句柄。
/// 首次调用通过 platforms 构造注册表，后续返回 Clone；旧锁污染时先清理运行时再恢复。
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
