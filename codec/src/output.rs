/// 平台调用成功后的有序文本输出。
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
