//! 节点调用契约、描述信息与生命周期调度。
//!
//! 此处不包含具体通信实现；属性宏和节点 crate 只需通过公开类型接入统一运行时。

mod contract;
mod descriptor;
mod lifecycle;
mod node;

pub use contract::{
    CLOSE_DEVICE_OPERATION, GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION,
    GET_PORT_NO_OPERATION, INIT_DEVICE_OPERATION, Invocation, InvocationOutput,
};
pub use descriptor::{NodeDescriptor, OperationDescriptor, ParameterDescriptor};
pub use lifecycle::NodeRuntime;
pub use node::DeviceNode;
