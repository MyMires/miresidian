//! 完整 ABI 请求的私有 JSON 契约；不参与平台适配或业务调度。

use serde::{Deserialize, Deserializer};

use crate::{MiresidianError, MiresidianResult};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InvocationRequest {
    pub function: String,
    #[serde(default)]
    pub config: Vec<String>,
    #[serde(default)]
    pub input: Vec<String>,
    // Option 默认会接受显式 null；此字段只允许缺省或真正的 u32 整数。
    #[serde(default, deserialize_with = "deserialize_platform")]
    pub platform: Option<u32>,
}

fn deserialize_platform<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    u32::deserialize(deserializer).map(Some)
}

pub(super) fn parse(text: &str) -> MiresidianResult<InvocationRequest> {
    let invalid = |message: String| MiresidianError::InvalidArgument {
        field: "request_json".to_owned(),
        message,
    };
    // serde 的结构体反序列化也可接收字段序列；ABI 明确只接受对象。
    if !text.trim_start().starts_with('{') {
        return Err(invalid("必须是 JSON 请求对象".to_owned()));
    }
    let request: InvocationRequest =
        serde_json::from_str(text).map_err(|error| invalid(format!("无效请求：{error}")))?;
    if request.function.is_empty() {
        return Err(invalid("function 必须为非空字符串".to_owned()));
    }
    Ok(request)
}
