//! 节点调用契约、描述信息与生命周期调度。
//!
//! 此处不包含具体通信实现；属性宏和节点 crate 只需通过公开类型接入统一运行时。

mod construction;
mod contract;
mod descriptor;
mod execution;
mod instances;
mod lifecycle;
#[cfg(feature = "instances")]
mod multi_channel;
mod node;
mod tasks;

pub use contract::{
    CHANNEL_REUSED_MESSAGE, CLOSE_ALL_DEVICES_OPERATION, CLOSE_DEVICE_OPERATION,
    GET_FRAMEWORK_VERSION_OPERATION, GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION,
    GET_PORT_NO_OPERATION, GET_PROJECT_VERSION_OPERATION, INIT_DEVICE_OPERATION, Invocation,
    SHARED_INVOCATION_MARKER,
};
pub use descriptor::{
    ExecutionMode, LeaseAccess, LeaseActivation, NodeDescriptor, OperationDescriptor,
    PLATFORM_ADAPTER_DEFINITION, PLATFORM_ADAPTER_NAME, ParameterDescriptor,
};
pub use instances::{ExecutionPolicy, InstanceBinding, InstanceMode, InstancePolicy};
pub use lifecycle::NodeRuntime;
#[doc(hidden)]
pub use node::validate_descriptor;
pub use node::{DeviceNode, NodeFuture};

#[doc(hidden)]
pub use construction::finish_initialization;

pub use execution::{CallContext, ExecutionLimits, shutdown_executor};
#[doc(hidden)]
pub use execution::{RuntimeSource, block_on};

pub use tasks::{TaskService, TaskSnapshot, TaskState};
