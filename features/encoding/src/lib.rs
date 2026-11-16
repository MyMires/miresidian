//! 显式格式编解码增强；由根框架 format feature 接入。
mod conversion;
pub use conversion::{
    EncodingInput, EncodingOutput, decode_encoded, decode_encoded_batch, encode_encoded,
    select_encoding, select_encoding_value,
};
