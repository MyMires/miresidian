//! 后续跨语言 ABI 允许使用的逻辑值类型。

/// 可在受支持运行时之间无损表达的单个标量值。
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceScalar {
    /// 有符号整数值。
    Signed(i64),
    /// 无符号整数值。
    Unsigned(u64),
    /// IEEE 754 浮点值。
    Float(f64),
    /// 布尔值。
    Boolean(bool),
    /// UTF-8 文本值。
    Text(String),
    /// 不透明字节序列。
    Bytes(Vec<u8>),
}

/// 平台安全值：空值、单个标量值或平坦的标量数组。
///
/// 此类型刻意不支持任意嵌套 Rust 结构，以保证后续 C 与 C# 绑定具有稳定、
/// 可文档化的表达形式。
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceValue {
    /// 缺失的可选值。
    Null,
    /// 单个标量值。
    Scalar(DeviceScalar),
    /// 同质或混合的平坦标量列表。
    Array(Vec<DeviceScalar>),
}
