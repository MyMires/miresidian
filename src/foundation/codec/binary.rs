//! HEX 与 Base64 的字节文本编解码。

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::MiresidianResult;

use super::invalid;

/// 解析紧凑 HEX，或以空白分隔的 `0x01 FF` 形式。
pub(super) fn decode_hex(value: &str) -> MiresidianResult<Vec<u8>> {
    let value = value.trim();
    let value = value
        .strip_prefix("hex:")
        .or_else(|| value.strip_prefix("HEX:"))
        .unwrap_or(value);
    if value.is_empty() {
        return Ok(Vec::new());
    }

    let compact = if value.chars().any(char::is_whitespace) {
        let mut compact = String::new();
        for item in value.split_whitespace() {
            let item = item
                .strip_prefix("0x")
                .or_else(|| item.strip_prefix("0X"))
                .unwrap_or(item);
            if item.is_empty() || item.len() > 2 {
                return Err(invalid("数据", "HEX 的每个字节必须是 1 或 2 位十六进制数"));
            }
            if item.len() == 1 {
                compact.push('0');
            }
            compact.push_str(item);
        }
        compact
    } else {
        value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
            .unwrap_or(value)
            .to_owned()
    };

    if compact.len() % 2 != 0 {
        return Err(invalid("数据", "HEX 文本必须包含完整的字节对"));
    }
    hex::decode(&compact).map_err(|_| invalid("数据", "HEX 文本格式错误"))
}

/// 按空白分隔的双位大写 HEX 格式化字节，便于人工阅读与回填。
pub(super) fn encode_hex(value: &[u8]) -> String {
    let compact = hex::encode_upper(value);
    compact
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| std::str::from_utf8(chunk).expect("十六进制文本只能包含 ASCII"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 解析标准 Base64；允许可选的 `base64:` 前缀。
pub(super) fn decode_base64(value: &str) -> MiresidianResult<Vec<u8>> {
    let value = value.trim();
    let value = value
        .strip_prefix("base64:")
        .or_else(|| value.strip_prefix("BASE64:"))
        .unwrap_or(value);
    STANDARD
        .decode(value)
        .map_err(|_| invalid("数据", "Base64 文本格式错误"))
}

/// 使用带前缀的标准 Base64 作为平台侧字节输出的规范形式。
pub(super) fn encode_base64(value: &[u8]) -> String {
    format!("base64:{}", STANDARD.encode(value))
}
