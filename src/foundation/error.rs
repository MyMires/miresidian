//! Miresidian 节点统一返回的错误类型。

use std::fmt::{Display, Formatter};

/// 节点实现和运行时使用的结果类型。
pub type MiresidianResult<T> = Result<T, MiresidianError>;

/// 可在跨语言边界表达的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiresidianError {
    /// 请求的操作不在节点契约中。
    InvalidOperation(String),
    /// 输入或配置值无法满足已声明的契约。
    InvalidArgument { field: String, message: String },
    /// 在成功初始化前请求了节点自定义操作。
    NotInitialized,
    /// 调用方传入的 Config 未对应任何已创建的通道对象。
    UnknownChannel(String),
    /// 节点未实现某项可选能力。
    NotSupported(String),
    /// 驱动特有的运行失败。
    Failure(String),
}

impl Display for MiresidianError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOperation(operation) => write!(formatter, "未知操作: {operation}"),
            Self::InvalidArgument { field, message } => {
                write!(formatter, "参数 {field} 无效: {message}")
            }
            Self::NotInitialized => formatter.write_str("设备尚未初始化"),
            Self::UnknownChannel(config) => {
                write!(formatter, "未找到 Config 对应的通道: [{config}]")
            }
            Self::NotSupported(capability) => write!(formatter, "不支持的能力: {capability}"),
            Self::Failure(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for MiresidianError {}
