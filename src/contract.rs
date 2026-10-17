//! 运行时与后续 FFI 边界共用的文本调用类型。

/// 返回可调用操作签名的兼容操作。
pub const GET_FUNCTIONS_OPERATION: &str = "GetFunctions";
/// 返回初始化配置元数据的兼容操作。
pub const GET_INIT_CONFIG_OPERATION: &str = "GetInitConfig";
/// 探测节点端口、但不保留节点实例的兼容操作。
pub const GET_PORT_NO_OPERATION: &str = "GetPortNO";
/// 创建并初始化当前节点的兼容操作。
pub const INIT_DEVICE_OPERATION: &str = "InitDevice";
/// 关闭并释放当前节点的兼容操作。
pub const CLOSE_DEVICE_OPERATION: &str = "CloseDevice";

/// 对 Miresidian 运行时发起的一次调用请求。
///
/// 本层刻意将 `config` 和 `input` 保留为文本，以便直接映射既有 C# 的
/// `CallFunction` 调用约定。后续由生成的包装代码负责验证并转换为 Rust 类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// 请求的操作名称，例如 `InitDevice` 或 `Query`。
    pub operation: String,
    /// 与本次调用关联的配置值。
    pub config: Vec<String>,
    /// 与本次调用关联的输入值。
    pub input: Vec<String>,
}

impl Invocation {
    /// 由平台侧的文本值列表创建调用请求。
    pub fn new(
        operation: impl Into<String>,
        config: impl Into<Vec<String>>,
        input: impl Into<Vec<String>>,
    ) -> Self {
        Self {
            operation: operation.into(),
            config: config.into(),
            input: input.into(),
        }
    }
}

/// 返回给调用方的文本结果。
///
/// C ABI 将负责把此类型转换为由调用方约定管理的 C 安全响应缓冲区；
/// Rust 集合绝不直接穿过该 ABI 边界。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvocationOutput {
    values: Vec<String>,
}

impl InvocationOutput {
    /// 由有序结果值创建输出。
    pub fn new(values: impl Into<Vec<String>>) -> Self {
        Self {
            values: values.into(),
        }
    }

    /// 返回空的成功输出。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 借用有序结果值。
    pub fn as_slice(&self) -> &[String] {
        &self.values
    }

    /// 消费输出并取得其有序结果值。
    pub fn into_values(self) -> Vec<String> {
        self.values
    }
}

impl From<Vec<String>> for InvocationOutput {
    fn from(values: Vec<String>) -> Self {
        Self::new(values)
    }
}
