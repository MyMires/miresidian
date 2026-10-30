//! Shared runtime orchestration, lifecycle transactions and instance isolation.
mod instance;
mod management;
mod routing;
mod service;
pub(super) use instance::Instance;
pub(in crate::runtime) use instance::{catch_node, cleanup_node};
pub use service::NodeRuntime;
