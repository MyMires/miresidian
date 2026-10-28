//! Miresidian 节点统一返回的错误类型。

use std::fmt::{Display, Formatter};

/// 节点实现和运行时使用的结果类型。
pub type MiresidianResult<T> = Result<T, MiresidianError>;

/// 与平台无关的错误类别；C ABI 分别映射为状态码 1、2、3。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// 请求参数、操作或平台选择无效。
    InvalidArgument,
    /// 生命周期、能力、租约或业务执行失败。
    OperationFailed,
    /// 节点 panic、状态污染或框架内部错误。
    InternalError,
}

/// 可在跨语言边界表达的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiresidianError {
    /// 请求的操作不在节点契约中。
    InvalidOperation(String),
    /// 输入或配置值无法满足已声明的契约。
    InvalidArgument { field: String, message: String },
    /// 平台数值使用了未分配的保留编号。
    InvalidPlatformId(u32),
    /// 平台标识合法，但未注册对应的适配器。
    UnknownPlatformAdapter(String),
    /// 在成功初始化前请求了节点自定义操作。
    NotInitialized,
    /// 调用方传入的 Config 未对应任何已创建的通道对象。
    UnknownChannel(String),
    /// 节点未实现某项可选能力。
    NotSupported(String),
    /// 节点发生可捕获的 panic；对应通道必须关闭后重新初始化。
    NodePanicked { operation: String },
    /// 节点锁已污染；对象仅允许清理。
    NodeStatePoisoned,
    /// 租约会话已失效；设备必须关闭后重新初始化。
    LeaseInvalidated(String),
    /// 框架边界捕获的未预期内部错误。
    Internal(String),
    /// 增加诊断上下文，同时保留底层错误的类别和稳定代码。
    Context { context: String, source: Box<Self> },
    /// 驱动特有的运行失败。
    Failure(String),
}

impl MiresidianError {
    /// 返回平台无关的错误分类。适配器应据此映射状态，不解析诊断消息。
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::InvalidOperation(_)
            | Self::InvalidArgument { .. }
            | Self::InvalidPlatformId(_)
            | Self::UnknownPlatformAdapter(_) => ErrorCategory::InvalidArgument,
            Self::NotInitialized
            | Self::UnknownChannel(_)
            | Self::NotSupported(_)
            | Self::LeaseInvalidated(_)
            | Self::Failure(_) => ErrorCategory::OperationFailed,
            Self::NodePanicked { .. } | Self::NodeStatePoisoned | Self::Internal(_) => {
                ErrorCategory::InternalError
            }
            Self::Context { source, .. } => source.category(),
        }
    }

    /// 返回稳定机器代码；文字、语言和上下文变化均不改变此值。
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidOperation(_) => "INVALID_OPERATION",
            Self::InvalidArgument { .. } => "INVALID_ARGUMENT",
            Self::InvalidPlatformId(_) => "INVALID_PLATFORM_ID",
            Self::UnknownPlatformAdapter(_) => "UNKNOWN_PLATFORM_ADAPTER",
            Self::NotInitialized => "NOT_INITIALIZED",
            Self::UnknownChannel(_) => "UNKNOWN_CHANNEL",
            Self::NotSupported(_) => "NOT_SUPPORTED",
            Self::NodePanicked { .. } => "NODE_PANICKED",
            Self::NodeStatePoisoned => "NODE_STATE_POISONED",
            Self::LeaseInvalidated(_) => "LEASE_INVALIDATED",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Failure(_) => "OPERATION_FAILED",
            Self::Context { source, .. } => source.code(),
        }
    }

    /// 附加通道或操作诊断，保留用于自动判断的类别与代码。
    pub fn with_context(self, context: impl Into<String>) -> Self {
        Self::Context {
            context: context.into(),
            source: Box::new(self),
        }
    }
}

impl Display for MiresidianError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOperation(operation) => write!(formatter, "未知操作: {operation}"),
            Self::InvalidArgument { field, message } => {
                write!(formatter, "参数 {field} 无效: {message}")
            }
            Self::InvalidPlatformId(value) => {
                write!(formatter, "参数 platform 不是有效的平台枚举值: {value}")
            }
            Self::UnknownPlatformAdapter(platform) => {
                write!(formatter, "未注册的平台适配器 {platform}")
            }
            Self::NotInitialized => formatter.write_str("设备尚未初始化"),
            Self::UnknownChannel(config) => {
                write!(formatter, "未找到 Config 对应的通道: [{config}]")
            }
            Self::NotSupported(capability) => write!(formatter, "不支持的能力: {capability}"),
            Self::NodePanicked { operation } => {
                write!(
                    formatter,
                    "节点在 {operation} 时发生异常；请关闭故障通道后重新初始化"
                )
            }
            Self::Failure(message) => formatter.write_str(message),
            Self::Internal(message) => formatter.write_str(message),
            Self::NodeStatePoisoned => {
                formatter.write_str("节点状态锁已损坏；请关闭通道后重新初始化")
            }
            Self::LeaseInvalidated(message) => write!(
                formatter,
                "租约已失效，通道停止访问设备；请关闭后重新初始化: {message}"
            ),
            Self::Context { context, source } => write!(formatter, "{context}: {source}"),
        }
    }
}

impl std::error::Error for MiresidianError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Context { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
