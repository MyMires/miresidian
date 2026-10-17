//! 平台文本值与 Rust 常用类型之间的统一转换。
//!
//! C ABI 只传递 UTF-8 字符串数组。属性宏在生成调用代码时通过本模块将文本转换为
//! 驱动方法的强类型参数，并将返回值稳定地转换回字符串数组。

use crate::{DriverError, DriverResult, InvocationOutput};

/// 可由测试平台文本构造的输入或配置类型。
pub trait DeviceInput: Sized {
    /// 从单个已规范化的文本值转换。
    fn parse_device_text(value: &str, field: &str) -> DriverResult<Self>;
}

/// 可安全返回给测试平台的驱动方法结果类型。
pub trait DeviceOutput {
    /// 转换为有序文本输出。
    fn into_device_output(self) -> DriverResult<InvocationOutput>;
}

/// 解析初始化配置；缺失或空白项目会使用属性声明的默认值。
pub fn parse_config<T: DeviceInput>(
    values: &[String],
    index: usize,
    field: &str,
    default: Option<&str>,
) -> DriverResult<T> {
    T::parse_device_text(value_at(values, index, field, default)?, field)
}

/// 解析设备方法输入；没有默认值时，缺失或空白项目会报错。
pub fn parse_input<T: DeviceInput>(
    values: &[String],
    index: usize,
    field: &str,
    default: Option<&str>,
) -> DriverResult<T> {
    T::parse_device_text(value_at(values, index, field, default)?, field)
}

/// 将驱动方法的强类型返回值转换为 ABI 输出。
pub fn into_invocation_output<T: DeviceOutput>(value: T) -> DriverResult<InvocationOutput> {
    value.into_device_output()
}

fn value_at<'a>(
    values: &'a [String],
    index: usize,
    field: &str,
    default: Option<&'a str>,
) -> DriverResult<&'a str> {
    let value = values
        .get(index)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let value = value
        .and_then(|value| {
            value
                .strip_prefix(&format!("{field}:"))
                .map(str::trim)
                .or(Some(value))
        })
        .filter(|value| !value.is_empty());

    value
        .or(default)
        .ok_or_else(|| DriverError::InvalidArgument {
            field: field.to_owned(),
            message: "不能为空".to_owned(),
        })
}

impl DeviceInput for String {
    fn parse_device_text(value: &str, _field: &str) -> DriverResult<Self> {
        Ok(value.to_owned())
    }
}

impl DeviceInput for bool {
    fn parse_device_text(value: &str, field: &str) -> DriverResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "是" | "真" => Ok(true),
            "false" | "0" | "否" | "假" => Ok(false),
            _ => Err(invalid(field, "必须是布尔值")),
        }
    }
}

macro_rules! numeric_input {
    ($($type:ty),+ $(,)?) => {$(
        impl DeviceInput for $type {
            fn parse_device_text(value: &str, field: &str) -> DriverResult<Self> {
                value.trim().parse::<$type>().map_err(|_| invalid(field, "数值格式错误"))
            }
        }
    )+};
}

numeric_input!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl<T: DeviceInput> DeviceInput for Vec<T> {
    fn parse_device_text(value: &str, field: &str) -> DriverResult<Self> {
        if value.trim().is_empty() {
            return Ok(Vec::new());
        }
        value
            .split('|')
            .map(|item| T::parse_device_text(item.trim(), field))
            .collect()
    }
}

impl<T: DeviceInput> DeviceInput for Option<T> {
    fn parse_device_text(value: &str, field: &str) -> DriverResult<Self> {
        if value.trim().is_empty() {
            Ok(None)
        } else {
            T::parse_device_text(value, field).map(Some)
        }
    }
}

impl DeviceOutput for () {
    fn into_device_output(self) -> DriverResult<InvocationOutput> {
        Ok(InvocationOutput::empty())
    }
}

impl DeviceOutput for String {
    fn into_device_output(self) -> DriverResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![self]))
    }
}

impl DeviceOutput for bool {
    fn into_device_output(self) -> DriverResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![self.to_string()]))
    }
}

macro_rules! scalar_output {
    ($($type:ty),+ $(,)?) => {$(
        impl DeviceOutput for $type {
            fn into_device_output(self) -> DriverResult<InvocationOutput> {
                Ok(InvocationOutput::new(vec![self.to_string()]))
            }
        }
    )+};
}

scalar_output!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl<T: ToString> DeviceOutput for Vec<T> {
    fn into_device_output(self) -> DriverResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![
            self.into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join("|"),
        ]))
    }
}

impl<A: DeviceOutput, B: DeviceOutput> DeviceOutput for (A, B) {
    fn into_device_output(self) -> DriverResult<InvocationOutput> {
        let mut values = self.0.into_device_output()?.into_values();
        values.extend(self.1.into_device_output()?.into_values());
        Ok(InvocationOutput::new(values))
    }
}

impl<A: DeviceOutput, B: DeviceOutput, C: DeviceOutput> DeviceOutput for (A, B, C) {
    fn into_device_output(self) -> DriverResult<InvocationOutput> {
        let mut values = self.0.into_device_output()?.into_values();
        values.extend(self.1.into_device_output()?.into_values());
        values.extend(self.2.into_device_output()?.into_values());
        Ok(InvocationOutput::new(values))
    }
}

fn invalid(field: &str, message: &str) -> DriverError {
    DriverError::InvalidArgument {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}
