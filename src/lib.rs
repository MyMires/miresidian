//! Miresidian 是供测试平台加载设备节点的运行时框架。
//!
//! 当前基础层定义稳定的调用契约与生命周期行为；属性宏和 C ABI 将在此基础上实现。

mod foundation;
pub mod interop;
mod runtime;

pub use ffi::{
    MIRESIDIAN_STATUS_INTERNAL_ERROR, MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK,
    MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianOutputV1, MiresidianStatus,
};
pub use foundation::{
    DeviceInput, DeviceOutput, DeviceScalar, DeviceValue, DriverError, DriverResult,
    into_invocation_output, parse_config, parse_input,
};
pub use interop::{ffi, ffi_runtime};
/// 供具体节点使用的属性宏。
pub use miresidian_macros::{config, input, node, node_impl, operation, output};
pub use runtime::{
    CLOSE_DEVICE_OPERATION, DeviceNode, GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION,
    GET_PORT_NO_OPERATION, INIT_DEVICE_OPERATION, Invocation, InvocationOutput, NodeDescriptor,
    NodeRuntime, OperationDescriptor, ParameterDescriptor,
};
