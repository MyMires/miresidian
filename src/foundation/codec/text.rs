//! ASCII、GBK、UTF-8 与 UTF-16LE（Unicode）的文本—字节转换。

use encoding_rs::GBK;

use crate::MiresidianResult;

use super::{ByteTextFormat, invalid};

/// 将 Rust UTF-8 字符串编码为指定文本格式的原始字节。
pub(super) fn encode_text(value: &str, format: ByteTextFormat) -> MiresidianResult<Vec<u8>> {
    match format {
        ByteTextFormat::Ascii => {
            if !value.is_ascii() {
                return Err(invalid("数据", "ASCII 不能表示非 ASCII 字符"));
            }
            Ok(value.as_bytes().to_vec())
        }
        ByteTextFormat::Gbk => {
            let (bytes, _, had_errors) = GBK.encode(value);
            if had_errors {
                return Err(invalid("数据", "GBK 不能无损表示输入文本"));
            }
            Ok(bytes.into_owned())
        }
        ByteTextFormat::Utf8 => Ok(value.as_bytes().to_vec()),
        ByteTextFormat::Utf16Le => Ok(value
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>()),
        ByteTextFormat::Hex | ByteTextFormat::Base64 => {
            unreachable!("二进制格式由 binary 模块处理")
        }
    }
}

/// 将指定文本格式的原始字节解码为 Rust UTF-8 字符串。
pub(super) fn decode_text(value: &[u8], format: ByteTextFormat) -> MiresidianResult<String> {
    match format {
        ByteTextFormat::Ascii => {
            if value.iter().any(|byte| !byte.is_ascii()) {
                return Err(invalid("数据", "字节序列不是有效 ASCII"));
            }
            String::from_utf8(value.to_vec()).map_err(|_| invalid("数据", "字节序列不是有效 ASCII"))
        }
        ByteTextFormat::Gbk => {
            let (text, _, had_errors) = GBK.decode(value);
            if had_errors {
                return Err(invalid("数据", "字节序列不是有效 GBK"));
            }
            Ok(text.into_owned())
        }
        ByteTextFormat::Utf8 => {
            String::from_utf8(value.to_vec()).map_err(|_| invalid("数据", "字节序列不是有效 UTF-8"))
        }
        ByteTextFormat::Utf16Le => decode_utf16_le(value),
        ByteTextFormat::Hex | ByteTextFormat::Base64 => {
            unreachable!("二进制格式由 binary 模块处理")
        }
    }
}

fn decode_utf16_le(value: &[u8]) -> MiresidianResult<String> {
    if !value.len().is_multiple_of(2) {
        return Err(invalid("数据", "Unicode（UTF-16LE）字节数必须为偶数"));
    }
    let units = value
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
    String::from_utf16(&units.collect::<Vec<_>>())
        .map_err(|_| invalid("数据", "字节序列不是有效 Unicode（UTF-16LE）"))
}
