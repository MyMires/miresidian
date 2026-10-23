//! 基础标量与标准集合元素的强类型转换。

use crate::{InvocationOutput, MiresidianResult};

use super::{ByteTextFormat, DeviceInput, DeviceOutput, invalid};

impl DeviceOutput for () {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::empty())
    }
}

impl DeviceInput for String {
    fn parse_device_text(value: &str, _field: &str) -> MiresidianResult<Self> {
        Ok(value.to_owned())
    }
}

impl DeviceOutput for String {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![self]))
    }
}

impl DeviceInput for bool {
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "是" | "真" => Ok(true),
            "false" | "0" | "否" | "假" => Ok(false),
            _ => Err(invalid(field, "必须是布尔值")),
        }
    }
}

impl DeviceOutput for bool {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![self.to_string()]))
    }
}

impl DeviceInput for char {
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
        let mut characters = value.chars();
        let Some(character) = characters.next() else {
            return Err(invalid(field, "必须是单个字符"));
        };
        if characters.next().is_some() {
            return Err(invalid(field, "必须是单个字符"));
        }
        Ok(character)
    }
}

impl DeviceOutput for char {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![self.to_string()]))
    }
}

fn parse_radix(value: &str, field: &str) -> MiresidianResult<Option<u128>> {
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
        return Ok(None);
    };
    if digits.is_empty() {
        return Err(invalid(field, "进制数值缺少数字部分"));
    }
    u128::from_str_radix(digits, radix)
        .map(Some)
        .map_err(|_| invalid(field, "进制数值格式错误"))
}

macro_rules! integer {
    ($($type:ty),+ $(,)?) => {$(
        impl DeviceInput for $type {
            fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
                if let Some(value) = parse_radix(value, field)? {
                    return <$type>::try_from(value).map_err(|_| invalid(field, "数值超出类型范围"));
                }
                value.trim().parse::<$type>().map_err(|_| invalid(field, "数值格式错误"))
            }
        }

        impl DeviceOutput for $type {
            fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
                Ok(InvocationOutput::new(vec![self.to_string()]))
            }
        }
    )+};
}

integer!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

macro_rules! float {
    ($($type:ty),+ $(,)?) => {$(
        impl DeviceInput for $type {
            fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
                let value = match parse_radix(value, field)? {
                    Some(value) => value as $type,
                    None => value.trim().parse::<$type>().map_err(|_| invalid(field, "数值格式错误"))?,
                };
                if !value.is_finite() {
                    return Err(invalid(field, "浮点数必须是有限值"));
                }
                Ok(value)
            }
        }

        impl DeviceOutput for $type {
            fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
                if !self.is_finite() {
                    return Err(invalid("输出", "浮点数必须是有限值"));
                }
                Ok(InvocationOutput::new(vec![self.to_string()]))
            }
        }
    )+};
}

float!(f32, f64);

impl DeviceInput for ByteTextFormat {
    fn parse_device_text(value: &str, _field: &str) -> MiresidianResult<Self> {
        value.parse()
    }
}

impl DeviceOutput for ByteTextFormat {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![self.to_string()]))
    }
}
