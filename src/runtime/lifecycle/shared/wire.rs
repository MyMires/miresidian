//! 私有节点代理线契约；不跨 DLL 传递 Rust 对象或指针。
use crate::{MiresidianError, MiresidianResult};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) enum Request {
    Initialize {
        contract: Vec<String>,
        config: Vec<String>,
        input: Vec<String>,
    },
    Validate,
    Invoke {
        operation: String,
        input: Vec<String>,
        deadline_ms: Option<u64>,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Close {
    pub input: Vec<String>,
}

// 保留全部错误类别、上下文和稳定代码，不把远端错误降级为字符串 Failure。
#[derive(Serialize, Deserialize)]
pub(super) enum Error {
    InvalidOperation(String),
    InvalidArgument { field: String, message: String },
    InvalidPlatformId(u32),
    UnknownPlatformAdapter(String),
    NotInitialized,
    UnknownChannel(String),
    NotSupported(String),
    NodePanicked { operation: String },
    NodeStatePoisoned,
    LeaseInvalidated(String),
    Internal(String),
    Context { context: String, source: Box<Error> },
    Failure(String),
}

macro_rules! convert {
    ($from:ident, $to:ident, $value:ident) => {
        match $value {
            $from::InvalidOperation(v) => $to::InvalidOperation(v),
            $from::InvalidArgument { field, message } => $to::InvalidArgument { field, message },
            $from::InvalidPlatformId(v) => $to::InvalidPlatformId(v),
            $from::UnknownPlatformAdapter(v) => $to::UnknownPlatformAdapter(v),
            $from::NotInitialized => $to::NotInitialized,
            $from::UnknownChannel(v) => $to::UnknownChannel(v),
            $from::NotSupported(v) => $to::NotSupported(v),
            $from::NodePanicked { operation } => $to::NodePanicked { operation },
            $from::NodeStatePoisoned => $to::NodeStatePoisoned,
            $from::LeaseInvalidated(v) => $to::LeaseInvalidated(v),
            $from::Internal(v) => $to::Internal(v),
            $from::Context { context, source } => $to::Context {
                context,
                source: Box::new((*source).into()),
            },
            $from::Failure(v) => $to::Failure(v),
        }
    };
}
impl From<MiresidianError> for Error {
    fn from(value: MiresidianError) -> Self {
        convert!(MiresidianError, Error, value)
    }
}
impl From<Error> for MiresidianError {
    fn from(value: Error) -> Self {
        convert!(Error, MiresidianError, value)
    }
}

pub(super) fn encode(
    result: MiresidianResult<Vec<String>>,
) -> Result<Vec<u8>, crate::lease::LeaseError> {
    serde_json::to_vec(&result.map_err(Error::from))
        .map_err(|error| crate::lease::LeaseError::LocalIpc(error.to_string()))
}
pub(super) fn decode(bytes: &[u8]) -> MiresidianResult<Vec<String>> {
    serde_json::from_slice::<Result<Vec<String>, Error>>(bytes)
        .map_err(|error| MiresidianError::Internal(format!("节点代理响应无效: {error}")))?
        .map_err(Into::into)
}
