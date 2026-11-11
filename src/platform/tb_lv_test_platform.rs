//! TB LV Test Platform 的兼容适配器。
//!
//! 该平台以 `Input[0] = "共享"` 表示共享调用，并用英文逗号编码单通道参数。
//! 此约定仅保留在适配器中；唯一一份 Config 原样保留，全部共享输入使用其定位的同一对象。

use crate::{
    Invocation, InvocationOutput, MiresidianResult, MultiChannelInvocation, Platform,
    PlatformAdapter, RuntimeInvocation, RuntimeResponse, SHARED_INVOCATION_MARKER,
};

/// TB LV Test Platform 的稳定平台标识。
pub const TB_LV_TEST_PLATFORM: &str = "tb_lv_test_platform";

/// 当前 TB LV Test Platform 的文本调用适配器。
#[derive(Debug, Default, Clone, Copy)]
pub struct TbLvTestPlatformAdapter;

impl PlatformAdapter for TbLvTestPlatformAdapter {
    fn format_parameter(&self, parameter: &crate::ParameterDescriptor) -> MiresidianResult<String> {
        if !parameter.has_auto_options() {
            return Ok(parameter.definition.to_owned());
        }
        let mut labels = std::collections::HashMap::new();
        let mut choices = Vec::new();
        for (index, option) in parameter.enum_options.iter().enumerate() {
            for text in [option.value, option.display] {
                if text.trim().is_empty()
                    || text.trim() != text
                    || text
                        .chars()
                        .any(|c| c.is_control() || ":/-|\\,()".contains(c))
                    || labels.insert(text, index).is_some_and(|old| old != index)
                {
                    return Err(crate::MiresidianError::InvalidArgument {
                        field: parameter.name.to_owned(),
                        message: "枚举候选不符合 TB 参数格式或存在歧义".to_owned(),
                    });
                }
            }
            choices.push(if option.value == option.display {
                option.value.to_owned()
            } else {
                format!("{}-{}", option.value, option.display)
            });
        }
        Ok(format!("{}:{}", parameter.name, choices.join("/")))
    }

    fn normalize_parameter(
        &self,
        parameter: &crate::ParameterDescriptor,
        value: &str,
    ) -> MiresidianResult<String> {
        if parameter.has_auto_options() {
            self.format_parameter(parameter)?;
        }
        let value = crate::normalize_value_with_definition(
            &[value.to_owned()],
            0,
            parameter.name,
            parameter.definition,
            None,
        )?;
        crate::normalize_enum_text(&value, parameter.enum_options, parameter.name)
    }

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
        Ok(RuntimeInvocation::MultiChannel(
            MultiChannelInvocation::new(invocation.operation, invocation.config, slots),
        ))
    }

    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(output) => Ok(output),
            RuntimeResponse::MultiChannel(response) => Ok(InvocationOutput::new(
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
