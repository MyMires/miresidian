//! 节点与运行时共同使用的基础类型和文本转换。
//!
//! 本模块将跨语言值模型、错误模型及文本参数转换集中管理，避免通信实现和运行时调度
//! 直接耦合具体数据表示。

mod conversion;
mod error;
mod value;

pub use conversion::{
    DeviceInput, DeviceOutput, into_invocation_output, parse_config, parse_input,
};
pub use error::{DriverError, DriverResult};
pub use value::{DeviceScalar, DeviceValue};
