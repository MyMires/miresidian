//! 显式格式转换。字节保持原样；其余内置类型采用 UTF-8 规范文本作为中间表示。
//! 本模块只在 format feature 下公开，不改变 DeviceInput / DeviceOutput 的默认规则。
use miresidian_codec::{
    ByteTextFormat, DeviceInput, DeviceOutput, MiresidianError, MiresidianResult,
};

fn invalid(field: &str, message: &str) -> MiresidianError {
    MiresidianError::InvalidArgument {
        field: field.into(),
        message: message.into(),
    }
}

/// 可从显式编码解码后的字节构造的业务类型。自定义类型须自行明确其字节契约。
pub trait FormatInput: Sized {
    /// 是否将字节直接作为业务数据；其他类型采用规范 UTF-8 文本。
    const RAW_BYTES: bool = false;
    /// 从解码后的字节构造；内置文本值要求合法 UTF-8。
    fn from_format_bytes(bytes: Vec<u8>, field: &str) -> MiresidianResult<Self>;
}
/// 可按显式格式输出的业务类型。
pub trait FormatOutput {
    /// 是否将字节直接作为业务数据。
    const RAW_BYTES: bool = false;
    /// 产生待编码字节；内置非字节类型采用规范 UTF-8 文本。
    fn into_format_bytes(self) -> MiresidianResult<Vec<u8>>;
}

impl FormatInput for Vec<u8> {
    const RAW_BYTES: bool = true;
    fn from_format_bytes(bytes: Vec<u8>, _: &str) -> MiresidianResult<Self> {
        Ok(bytes)
    }
}
impl FormatOutput for Vec<u8> {
    const RAW_BYTES: bool = true;
    fn into_format_bytes(self) -> MiresidianResult<Vec<u8>> {
        Ok(self)
    }
}
impl FormatInput for String {
    fn from_format_bytes(bytes: Vec<u8>, field: &str) -> MiresidianResult<Self> {
        String::from_utf8(bytes).map_err(|_| invalid(field, "业务文本必须是合法 UTF-8"))
    }
}
impl FormatOutput for String {
    fn into_format_bytes(self) -> MiresidianResult<Vec<u8>> {
        Ok(self.into_bytes())
    }
}
macro_rules! text_types {
    ($($ty:ty),+ $(,)?) => {$(
        impl FormatInput for $ty {
            fn from_format_bytes(bytes: Vec<u8>, field: &str) -> MiresidianResult<Self> {
                Self::parse_device_text(&String::from_format_bytes(bytes, field)?, field)
            }
        }
        impl FormatOutput for $ty {
            fn into_format_bytes(self) -> MiresidianResult<Vec<u8>> { Ok(self.into_device_output()?.into_values().join("").into_bytes()) }
        }
    )+};
}
text_types!(
    bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

// 空白分隔的数值/布尔数组。字符串数组无无歧义元素边界，故不提供内置实现。
macro_rules! sequence_types {
    ($($ty:ty),+ $(,)?) => {$(
        impl FormatInput for Vec<$ty> {
            fn from_format_bytes(bytes: Vec<u8>, field: &str) -> MiresidianResult<Self> {
                String::from_format_bytes(bytes, field)?.split_whitespace()
                    .map(|item| <$ty>::parse_device_text(item, field)).collect()
            }
        }
        impl FormatOutput for Vec<$ty> {
            fn into_format_bytes(self) -> MiresidianResult<Vec<u8>> {
                Ok(self.into_iter()
                    .map(|value| value.into_device_output().map(|output| output.into_values().join("")))
                    .collect::<MiresidianResult<Vec<_>>>()?.join(" ").into_bytes())
            }
        }
    )+};
}
sequence_types!(
    bool, i8, i16, i32, i64, i128, isize, u16, u32, u64, u128, usize, f32, f64
);
impl<T: FormatInput> FormatInput for Option<T> {
    const RAW_BYTES: bool = T::RAW_BYTES;
    fn from_format_bytes(bytes: Vec<u8>, field: &str) -> MiresidianResult<Self> {
        if bytes.is_empty() {
            Ok(None)
        } else {
            T::from_format_bytes(bytes, field).map(Some)
        }
    }
}
impl<T: FormatOutput> FormatOutput for Option<T> {
    const RAW_BYTES: bool = T::RAW_BYTES;
    fn into_format_bytes(self) -> MiresidianResult<Vec<u8>> {
        self.map_or_else(|| Ok(Vec::new()), FormatOutput::into_format_bytes)
    }
}

/// 解析格式选择；缺失/空白采用第一项，只接受允许列表中的格式。
pub fn select_format(
    values: &[String],
    index: usize,
    field: &str,
    formats: &[ByteTextFormat],
) -> MiresidianResult<ByteTextFormat> {
    let value =
        miresidian_codec::normalize_value_with_definition(values, index, field, field, Some(""))?;
    select_format_value(&value, field, formats)
}

/// 校验一条格式选择值；空白使用允许列表首项，不解释批量分隔或执行策略。
/// `formats` 为空、格式未知或不在允许列表中时，返回归属于 `field` 的参数错误。
pub fn select_format_value(
    value: &str,
    field: &str,
    formats: &[ByteTextFormat],
) -> MiresidianResult<ByteTextFormat> {
    let first = formats
        .first()
        .copied()
        .ok_or_else(|| invalid(field, "格式列表不能为空"))?;
    if value.trim().is_empty() {
        return Ok(first);
    }
    let format = miresidian_codec::parse_byte_text_format(value)
        .map_err(|_| invalid(field, "未知的格式选择值"))?;
    if formats.contains(&format) {
        Ok(format)
    } else {
        Err(invalid(field, "格式不在允许列表中"))
    }
}
/// 按指定格式解码单条数据。
pub fn decode_formatted<T: FormatInput>(
    value: &str,
    format: ByteTextFormat,
    field: &str,
) -> MiresidianResult<T> {
    let decoded = miresidian_codec::decode_text_to_bytes(value, format)?;
    let bytes = if T::RAW_BYTES || matches!(format, ByteTextFormat::Hex | ByteTextFormat::Base64) {
        decoded
    } else {
        value.as_bytes().to_vec()
    };
    T::from_format_bytes(bytes, field)
}
/// 将业务结果按指定格式编码为一个完整输出槽位文本。
pub fn encode_formatted<T: FormatOutput>(
    value: T,
    format: ByteTextFormat,
) -> MiresidianResult<String> {
    let bytes = value.into_format_bytes()?;
    if T::RAW_BYTES || matches!(format, ByteTextFormat::Hex | ByteTextFormat::Base64) {
        miresidian_codec::encode_bytes_to_text(&bytes, format)
    } else {
        let text = String::from_format_bytes(bytes, "输出")?;
        miresidian_codec::decode_text_to_bytes(&text, format)?;
        Ok(text)
    }
}
/// 按同一种格式解码多条转义竖线记录的兼容便捷函数；不参与批次长度对齐或调用调度。
/// 空文本表示一条空记录；逐条格式选择由宏组合独立的 batch 与单条 format 接口。
///
/// 记录数由调用上下文保持；单条空结果与空集合不能仅凭一个输出字符串区分。
pub fn decode_formatted_batch<T: FormatInput>(
    value: &str,
    format: ByteTextFormat,
    field: &str,
) -> MiresidianResult<Vec<T>> {
    let records = if value.is_empty() {
        vec![String::new()]
    } else {
        <Vec<String>>::parse_device_text(value, field)?
    };
    records
        .into_iter()
        .map(|record| decode_formatted(&record, format, field))
        .collect()
}
