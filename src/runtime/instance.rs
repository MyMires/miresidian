//! 实例 crate 在根运行时中的策略与布局接入。

pub(crate) use miresidian_instance::{ConfigLayout, InstanceStore};
pub use miresidian_instance::{ExecutionPolicy, InstanceBinding, InstanceMode, InstancePolicy};
pub(crate) const HANDLE_DEFINITION: &str = "Handle";
