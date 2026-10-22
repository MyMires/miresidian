//! 节点与运行时共同使用的基础类型与数据编解码能力。
//!
//! 本模块将跨语言值模型、错误模型及数据编解码集中管理，避免通信实现和运行时调度直接耦合
//! 具体数据表示。

mod codec;
mod error;
mod value;

pub use codec::{
    ByteTextFormat, DeviceCollectionElement, DeviceInput, DeviceOutput, decode_text_to_bytes,
    decode_value, decode_value_with_definition, encode_bytes_to_text, encode_value,
    parse_byte_text_format, transcode_text,
};
pub use error::{MiresidianError, MiresidianResult};
pub use value::{DeviceScalar, DeviceValue};
