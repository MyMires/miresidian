//! 通过 Cargo 管理用户级工具；解析与进程执行分别维护。
mod execution;
mod options;

pub(super) use execution::{Action, execute};
pub(super) use options::parse;
