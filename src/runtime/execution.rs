//! Bounded execution, cooperative context and shared-library ownership.
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
