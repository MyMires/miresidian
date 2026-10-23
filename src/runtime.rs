//! 节点调用契约、描述信息与生命周期调度。
//!
//! 此处不包含具体通信实现；属性宏和节点 crate 只需通过公开类型接入统一运行时。

mod contract;
mod descriptor;
mod lifecycle;
mod node;
mod shared;

pub use contract::{
    CHANNEL_REUSED_MESSAGE, CLOSE_DEVICE_OPERATION, GET_FRAMEWORK_VERSION_OPERATION,
    GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION,
    GET_PROJECT_VERSION_OPERATION, INIT_DEVICE_OPERATION, Invocation, SHARED_INVOCATION_MARKER,
};
pub use descriptor::{ExecutionMode, NodeDescriptor, OperationDescriptor, ParameterDescriptor};
pub use lifecycle::NodeRuntime;
pub use node::DeviceNode;
