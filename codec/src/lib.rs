//! 面向全部节点的强类型值、字节与平台参数编解码。
//!
//! 该模块统一定义数据在 Rust 类型、平台字符串数组和原始字节之间的转换规则。通信节点不应
//! 自行实现 HEX、Base64 或字符编码逻辑；它们只需调用这里公开的函数。

mod binary;
mod collection;
mod definition;
mod error;
mod output;
mod scalar;
mod text;
mod value;

use std::{fmt, str::FromStr};

pub use error::{ErrorCategory, MiresidianError, MiresidianResult};
pub use output::InvocationOutput;
pub use value::{DeviceScalar, DeviceValue};

/// 原始字节与平台文本之间使用的明确数据格式。
///
/// 不提供含义依赖操作系统区域设置的 `ANSI`。需要单字节文本时使用 `Ascii`，需要中文兼容
/// 编码时使用 `Gbk`；两者的兼容关系和失败行为都是可预测的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteTextFormat {
    /// 严格 ASCII；只能表达 `0x00..=0x7F` 字节。
    Ascii,
    /// GBK 中文编码。
    Gbk,
    /// UTF-8 文本。
    Utf8,
    /// UTF-16 小端序；平台名称 `Unicode` 是它的别名。
    Utf16Le,
    /// 以十六进制文本表达原始字节。
    Hex,
    /// 以 Base64 文本表达原始字节。
    Base64,
}

impl fmt::Display for ByteTextFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Ascii => "ASCII",
            Self::Gbk => "GBK",
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "Unicode",
            Self::Hex => "HEX",
            Self::Base64 => "Base64",
        })
    }
}

impl FromStr for ByteTextFormat {
    type Err = MiresidianError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim();
        if normalized.eq_ignore_ascii_case("ANSI") {
            return Err(invalid(
                "数据格式",
                "不支持含义不确定的 ANSI；请使用 ASCII、GBK、UTF-8、Unicode、HEX 或 Base64",
            ));
        }
        if normalized.eq_ignore_ascii_case("ASCII") {
            return Ok(Self::Ascii);
        }
        if normalized.eq_ignore_ascii_case("GBK") {
            return Ok(Self::Gbk);
        }
        if normalized.eq_ignore_ascii_case("UTF-8") || normalized.eq_ignore_ascii_case("UTF8") {
            return Ok(Self::Utf8);
        }
        if normalized.eq_ignore_ascii_case("Unicode")
            || normalized.eq_ignore_ascii_case("UTF-16LE")
            || normalized.eq_ignore_ascii_case("UTF16LE")
        {
            return Ok(Self::Utf16Le);
        }
        if normalized.eq_ignore_ascii_case("HEX") {
            return Ok(Self::Hex);
        }
        if normalized.eq_ignore_ascii_case("Base64") {
            return Ok(Self::Base64);
        }
        Err(invalid(
            "数据格式",
            "仅支持 ASCII、GBK、UTF-8、Unicode、HEX 或 Base64",
        ))
    }
}

/// 解析平台传入的数据格式名称。
pub fn parse_byte_text_format(value: &str) -> MiresidianResult<ByteTextFormat> {
    value.parse()
}

/// 将某种数据格式的文本解码为原始字节。
pub fn decode_text_to_bytes(value: &str, format: ByteTextFormat) -> MiresidianResult<Vec<u8>> {
    match format {
        ByteTextFormat::Hex => binary::decode_hex(value),
        ByteTextFormat::Base64 => binary::decode_base64(value),
        text_format => text::encode_text(value, text_format),
    }
}

/// 将原始字节编码为指定数据格式的文本。
pub fn encode_bytes_to_text(value: &[u8], format: ByteTextFormat) -> MiresidianResult<String> {
    match format {
        ByteTextFormat::Hex => Ok(binary::encode_hex(value)),
        ByteTextFormat::Base64 => Ok(binary::encode_base64(value)),
        text_format => text::decode_text(value, text_format),
    }
}

/// 先按源格式解码、再按目标格式编码，实现无损数据转码。
pub fn transcode_text(
    value: &str,
    source: ByteTextFormat,
    target: ByteTextFormat,
) -> MiresidianResult<String> {
    encode_bytes_to_text(&decode_text_to_bytes(value, source)?, target)
}

/// 可由测试平台参数文本构造的输入或配置类型。
pub trait DeviceInput: Sized {
    /// 从单个已完成名称与默认值处理的文本值构造目标类型。
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self>;
}

/// 可安全返回给测试平台的驱动方法结果类型。
pub trait DeviceOutput {
    /// 转换为有序的平台输出文本。
    fn into_device_output(self) -> MiresidianResult<InvocationOutput>;
}

/// 可作为一维平台数组元素的类型。
///
/// 对标准类型已由框架实现。自定义业务类型若要使用 `Vec<T>` 参数或返回值，应实现本 trait，
/// 以明确其数组文本的解析和格式化规则。
pub trait DeviceCollectionElement: Sized {
    /// 将整个数组文本转换为有序元素。
    fn parse_device_collection(value: &str, field: &str) -> MiresidianResult<Vec<Self>>;

    /// 将有序元素转换为可无损回传的平台数组文本。
    fn format_device_collection(values: &[Self]) -> MiresidianResult<String>;
}

/// 从一个平台参数通道解码强类型值；空白值优先回退到属性声明的默认值。
///
/// `values` 可以是平台的 `Config[]` 或 `Input[]`。两者的文本规则完全相同，
/// 通道选择和索引分配由属性宏负责。
pub fn decode_value<T: DeviceInput>(
    values: &[String],
    index: usize,
    field: &str,
    default: Option<&str>,
) -> MiresidianResult<T> {
    T::parse_device_text(&definition::value_at(values, index, field, default)?, field)
}

/// 从平台参数通道提取带完整定义的规范化文本。
///
/// 该入口与宏生成的参数解码共用默认值、参数名前缀和显示值规则。运行时使用它生成
/// 稳定通道键，因此语义相同的 Config 文本会定位到同一已初始化通道。
pub fn normalize_value_with_definition(
    values: &[String],
    index: usize,
    field: &str,
    definition: &str,
    default: Option<&str>,
) -> MiresidianResult<String> {
    let default = (!definition::has_option_pairs(definition))
        .then_some(default)
        .flatten();
    let value = definition::value_at(values, index, field, default)?;
    Ok(definition::normalize_option_value(value, definition))
}

/// 从平台参数通道解码带完整平台描述的强类型值。
///
/// 属性宏使用此入口，使选项描述中的显示值在进入 `DeviceInput` 前规范化为实际值。
pub fn decode_value_with_definition<T: DeviceInput>(
    values: &[String],
    index: usize,
    field: &str,
    definition: &str,
    default: Option<&str>,
) -> MiresidianResult<T> {
    T::parse_device_text(
        &normalize_value_with_definition(values, index, field, definition, default)?,
        field,
    )
}

/// 将驱动方法的强类型返回值编码为平台 ABI 输出。
pub fn encode_value<T: DeviceOutput>(value: T) -> MiresidianResult<InvocationOutput> {
    value.into_device_output()
}

impl<T: DeviceCollectionElement> DeviceInput for Vec<T> {
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
        T::parse_device_collection(value, field)
    }
}

impl<T: DeviceCollectionElement> DeviceOutput for Vec<T> {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::new(vec![T::format_device_collection(
            &self,
        )?]))
    }
}

impl<T: DeviceInput> DeviceInput for Option<T> {
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
        if value.trim().is_empty() {
            Ok(None)
        } else {
            T::parse_device_text(value, field).map(Some)
        }
    }
}

impl<T: DeviceOutput> DeviceOutput for Option<T> {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        match self {
            Some(value) => value.into_device_output(),
            None => Ok(InvocationOutput::new(vec![String::new()])),
        }
    }
}

impl<A: DeviceOutput, B: DeviceOutput> DeviceOutput for (A, B) {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        let mut values = self.0.into_device_output()?.into_values();
        values.extend(self.1.into_device_output()?.into_values());
        Ok(InvocationOutput::new(values))
    }
}

impl<A: DeviceOutput, B: DeviceOutput, C: DeviceOutput> DeviceOutput for (A, B, C) {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        let mut values = self.0.into_device_output()?.into_values();
        values.extend(self.1.into_device_output()?.into_values());
        values.extend(self.2.into_device_output()?.into_values());
        Ok(InvocationOutput::new(values))
    }
}

impl<A: DeviceOutput, B: DeviceOutput, C: DeviceOutput, D: DeviceOutput> DeviceOutput
    for (A, B, C, D)
{
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        let mut values = self.0.into_device_output()?.into_values();
        values.extend(self.1.into_device_output()?.into_values());
        values.extend(self.2.into_device_output()?.into_values());
        values.extend(self.3.into_device_output()?.into_values());
        Ok(InvocationOutput::new(values))
    }
}

pub(crate) fn invalid(field: &str, message: impl Into<String>) -> MiresidianError {
    MiresidianError::InvalidArgument {
        field: field.to_owned(),
        message: message.into(),
    }
}
