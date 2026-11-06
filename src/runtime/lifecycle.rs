//! 共享运行时的调度入口；协调构造与关闭事务，并在实例边界隔离业务状态和异常。
mod instance;
mod management;
mod routing;
mod service;
#[cfg(feature = "lease")]
mod shared;
pub(super) use instance::Instance;
pub(in crate::runtime) use instance::{catch_node, cleanup_node};
pub use service::NodeRuntime;
