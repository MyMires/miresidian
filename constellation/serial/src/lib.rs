//! Miresidian 串口通信实例节点。
//!
//! 平台可调用方法集中在 `node`，串口连接与跨 DLL 租约位于 `transport`；
//! 平台契约的集成测试位于 crate 的 `tests/` 目录。此入口只负责声明模块边界和公开节点类型。

mod lease;
mod node;
mod transport;

pub use node::SerialNode;
