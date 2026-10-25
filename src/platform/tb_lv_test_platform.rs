//! TB LV Test Platform 的兼容适配器。
//!
//! 该平台以 `Input[0] = "共享"` 表示共享调用，并用英文逗号编码单通道参数。
//! 此约定仅保留在适配器中，运行时只接收结构化共享槽位。

use crate::{
    Invocation, InvocationOutput, MiresidianResult, Platform, PlatformAdapter, RuntimeInvocation,
    RuntimeResponse, SHARED_INVOCATION_MARKER, SharedInvocation,
};

/// TB LV Test Platform 的稳定平台标识。
pub const TB_LV_TEST_PLATFORM: &str = "tb_lv_test_platform";

/// 当前 TB LV Test Platform 的文本调用适配器。
#[derive(Debug, Default, Clone, Copy)]
pub struct TbLvTestPlatformAdapter;

impl PlatformAdapter for TbLvTestPlatformAdapter {
    fn platform(&self) -> Platform {
        Platform::TbLvTestPlatform
    }

    fn identifier(&self) -> &'static str {
        TB_LV_TEST_PLATFORM
    }

    fn decode(&self, invocation: Invocation) -> MiresidianResult<RuntimeInvocation> {
        if invocation
            .input
            .first()
            .is_none_or(|marker| marker != SHARED_INVOCATION_MARKER)
        {
            return Ok(RuntimeInvocation::Single(invocation.clone()));
        }
        let slots = invocation.input[1..]
            .iter()
            .map(|channel| {
                let channel = channel.trim();
                (!channel.is_empty()).then(|| {
                    channel
                        .split(",")
                        .map(|value| value.trim().to_owned())
                        .collect()
                })
            })
            .collect();
        Ok(RuntimeInvocation::Shared(SharedInvocation::new(
            invocation.operation.clone(),
            slots,
        )))
    }

    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(output) => Ok(output),
            RuntimeResponse::Shared(response) => Ok(InvocationOutput::new(
                response
                    .slots
                    .into_iter()
                    .map(|slot| {
                        slot.map(|output| output.into_values().join(","))
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>(),
            )),
        }
    }
}
