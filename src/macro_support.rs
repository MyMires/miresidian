//! 宏生成代码使用的内部支撑入口；由根框架与宏的精确版本共同维护。
//! 节点业务应使用公开重导出，不直接依赖此隐藏模块的布局。

pub use crate::runtime::{block_on, finish_initialization, validate_descriptor};
pub use inventory;

#[cfg(feature = "batch")]
pub use miresidian_batch::{batch_length, batch_outputs, batch_records, decode_batch};
