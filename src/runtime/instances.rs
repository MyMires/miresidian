//! 实例 crate 在根运行时中的策略与布局接入。

pub(crate) use miresidian_instances::{ConfigLayout, InstanceStore};
pub use miresidian_instances::{ExecutionPolicy, InstanceBinding, InstanceMode, InstancePolicy};
pub(crate) const HANDLE_DEFINITION: &str = "Handle";
