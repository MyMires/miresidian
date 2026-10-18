//! Miresidian 是供测试平台加载设备节点的运行时框架。
//!
//! 当前基础层定义稳定的调用契约与生命周期行为；属性宏和 C ABI 将在此基础上实现。
//!
//! # 节点声明示例
//!
//! 下例是一个最小可发布节点。驱动作者只声明状态与方法标注；函数列表、类型转换、生命周期
//! 调度和 DLL 的 C ABI 导出均由宏生成。
//!
//! ```no_run
//! use miresidian::DriverResult;
//!
//! #[miresidian::node("示例节点")]
//! #[derive(Default)]
//! struct ExampleNode;
//!
//! #[miresidian::node_impl]
//! impl ExampleNode {
//!     #[miresidian::output("状态")]
//!     fn init_device(&mut self) -> DriverResult<String> {
//!         Ok("已连接".to_owned())
//!     }
//!
//!     fn close_device(&mut self) -> DriverResult<()> {
//!         Ok(())
//!     }
//!
//!     #[miresidian::operation("回显")]
//!     #[miresidian::output("接收文本")]
//!     fn echo(&mut self, #[miresidian::input("发送文本")] text: String) -> DriverResult<String> {
//!         Ok(text)
//!     }
//! }
//! ```

mod foundation;
pub mod interop;
mod runtime;

/// 框架级数据编解码 API。
///
/// 属性宏生成的节点调度代码直接调用此模块，因此全部节点共用同一套类型、数组、字节和字符
/// 编码规则。根 crate 同时保留同名重导出，以兼容已有 Rust 调用代码。
pub mod codec {
    pub use crate::foundation::{
        ByteTextFormat, DeviceCollectionElement, DeviceInput, DeviceOutput, decode_text_to_bytes,
        decode_value, encode_bytes_to_text, encode_value, parse_byte_text_format, transcode_text,
    };
}

pub use ffi::{
    MIRESIDIAN_STATUS_INTERNAL_ERROR, MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK,
    MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianOutputV1, MiresidianStatus,
};
pub use foundation::{
    ByteTextFormat, DeviceCollectionElement, DeviceInput, DeviceOutput, DeviceScalar, DeviceValue,
    DriverError, DriverResult, decode_text_to_bytes, decode_value, encode_bytes_to_text,
    encode_value, parse_byte_text_format, transcode_text,
};
pub use interop::{ffi, ffi_runtime};
/// 供具体节点使用的属性宏。
pub use miresidian_macros::{config, input, node, node_impl, operation, output};
pub use runtime::{
    CLOSE_DEVICE_OPERATION, DeviceNode, GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION,
    GET_PORT_NO_OPERATION, INIT_DEVICE_OPERATION, Invocation, InvocationOutput, NodeDescriptor,
    NodeRuntime, OperationDescriptor, ParameterDescriptor,
};
