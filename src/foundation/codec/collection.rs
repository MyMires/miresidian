//! 平台一维数组的可逆竖线语法与整数范围展开。

use crate::MiresidianResult;

use super::{DeviceCollectionElement, DeviceInput, DeviceOutput, binary, invalid};

/// 按竖线分隔元素，并识别 `\|` 与 `\\` 转义。
fn split_items(value: &str, field: &str) -> MiresidianResult<Vec<String>> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    let mut item = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            if character != '|' && character != '\\' {
                return Err(invalid(field, "数组转义仅支持 \\| 和 \\\\"));
            }
            item.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '|' {
            items.push(std::mem::take(&mut item));
        } else {
            item.push(character);
        }
    }
    if escaped {
        return Err(invalid(field, "数组不能以未完成的反斜杠转义结尾"));
    }
    items.push(item);
    Ok(items)
}

fn format_items(values: impl IntoIterator<Item = String>) -> String {
    values
        .into_iter()
        .map(|value| value.replace('\\', "\\\\").replace('|', "\\|"))
        .collect::<Vec<_>>()
        .join("|")
}

fn parse_plain_collection<T: DeviceInput>(value: &str, field: &str) -> MiresidianResult<Vec<T>> {
    split_items(value, field)?
        .into_iter()
        .map(|item| T::parse_device_text(&item, field))
        .collect()
}

fn format_plain_collection<T: DeviceOutput + Clone>(values: &[T]) -> MiresidianResult<String> {
    values
        .iter()
        .cloned()
        .map(|value| {
            let output = value.into_device_output()?;
            if output.as_slice().len() != 1 {
                return Err(invalid("数组", "数组元素必须对应一个平台文本值"));
            }
            Ok(output
                .into_values()
                .into_iter()
                .next()
                .expect("已确认长度为一"))
        })
        .collect::<MiresidianResult<Vec<_>>>()
        .map(format_items)
}

fn parse_integral_collection<T>(
    value: &str,
    field: &str,
    parse: impl Fn(&str, &str) -> MiresidianResult<T>,
) -> MiresidianResult<Vec<T>> {
    let mut values = Vec::new();
    for item in split_items(value, field)? {
        let Some((start, end)) = range_bounds(&item, field)? else {
            values.push(parse(&item, field)?);
            continue;
        };
        if end < start {
            return Err(invalid(field, "数组范围必须按递增顺序书写，例如 1-5"));
        }
        if end - start > 65_535 {
            return Err(invalid(field, "数组范围最多展开 65536 个元素"));
        }
        for current in start..=end {
            values.push(parse(&current.to_string(), field)?);
        }
    }
    Ok(values)
}

fn range_bounds(value: &str, field: &str) -> MiresidianResult<Option<(u128, u128)>> {
    let value = value.trim();
    if value.starts_with('-') {
        return Ok(None);
    }
    let Some((start, end)) = value.split_once('-') else {
        return Ok(None);
    };
    if end.contains('-') || start.is_empty() || end.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        parse_unsigned(start, field)?,
        parse_unsigned(end, field)?,
    )))
}

fn parse_unsigned(value: &str, field: &str) -> MiresidianResult<u128> {
    let value = value.trim();
    let (radix, digits) = if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        (16, value)
    } else if let Some(value) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        (2, value)
    } else {
        (10, value)
    };
    u128::from_str_radix(digits, radix).map_err(|_| invalid(field, "数组范围不是有效的非负整数"))
}

macro_rules! plain_collection {
    ($($type:ty),+ $(,)?) => {$(
        impl DeviceCollectionElement for $type {
            fn parse_device_collection(value: &str, field: &str) -> MiresidianResult<Vec<Self>> {
                parse_plain_collection(value, field)
            }

            fn format_device_collection(values: &[Self]) -> MiresidianResult<String> {
                format_plain_collection(values)
            }
        }
    )+};
}

macro_rules! integral_collection {
    ($($type:ty),+ $(,)?) => {$(
        impl DeviceCollectionElement for $type {
            fn parse_device_collection(value: &str, field: &str) -> MiresidianResult<Vec<Self>> {
                parse_integral_collection(value, field, <$type>::parse_device_text)
            }

            fn format_device_collection(values: &[Self]) -> MiresidianResult<String> {
                format_plain_collection(values)
            }
        }
    )+};
}

plain_collection!(String, bool, char, f32, f64);
integral_collection!(i8, i16, i32, i64, i128, isize, u16, u32, u64, u128, usize);

impl DeviceCollectionElement for u8 {
    fn parse_device_collection(value: &str, field: &str) -> MiresidianResult<Vec<Self>> {
        if value
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("base64:")
        {
            return binary::decode_base64(value)
                .map_err(|_| invalid(field, "Base64 字节数组格式错误"));
        }
        parse_integral_collection(value, field, u8::parse_device_text)
    }

    fn format_device_collection(values: &[Self]) -> MiresidianResult<String> {
        Ok(binary::encode_base64(values))
    }
}
