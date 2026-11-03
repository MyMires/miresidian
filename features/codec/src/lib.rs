//! 显式格式编解码增强；由根框架 codec feature 接入。
mod conversion;
pub use conversion::{
    CodecInput, CodecOutput, decode_formatted, decode_formatted_batch, encode_formatted,
    select_format,
};
