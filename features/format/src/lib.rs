//! 显式格式编解码增强；由根框架 format feature 接入。
mod conversion;
pub use conversion::{
    FormatInput, FormatOutput, decode_formatted, decode_formatted_batch, encode_formatted,
    select_format,
};
