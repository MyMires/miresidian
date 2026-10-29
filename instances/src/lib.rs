//! 设备无关的实例策略、对象存储与多通道调度。

#[cfg(feature = "runtime")]
mod dispatch;
mod policy;
mod store;

#[cfg(feature = "runtime")]
pub use dispatch::invoke_multi_channel;
#[doc(hidden)]
pub use policy::ConfigLayout;
pub use policy::{ExecutionMode, ExecutionPolicy, InstanceBinding, InstanceMode, InstancePolicy};
pub use store::InstanceStore;
