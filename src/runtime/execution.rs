//! 有界请求调度、协作取消与执行器模块加载引用；业务池和生命周期池分别管理。
mod context;
mod limits;
mod module;
mod pool;
mod source;
pub use context::CallContext;
pub use limits::ExecutionLimits;
#[doc(hidden)]
pub use pool::block_on;
pub use pool::shutdown_executor;
pub(crate) use pool::{Activity, executor, lifecycle_executor};
pub use source::RuntimeSource;
