//! 元数据共用布局下的 Config 解析与对象关联键。

use super::{NodeRuntime, catch_node};
use crate::runtime::instance::ConfigLayout;
use crate::{
    ExecutionMode, INIT_DEVICE_OPERATION, Invocation, LeaseAccess, MiresidianError,
    MiresidianResult, Node, PLATFORM_ADAPTER_NAME, Platform, PlatformAdapter,
};
use crate::{InstanceMode, normalize_value_with_definition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InstanceKey {
    Config(Vec<String>),
    Handle(String),
}

impl InstanceKey {
    pub fn description(&self) -> String {
        match self {
            Self::Config(config) => config.join(","),
            Self::Handle(handle) => format!("Handle:{handle}"),
        }
    }
}

pub(super) struct ParsedConfig<'a> {
    pub node: &'a [String],
    pub access: LeaseAccess,
    pub platform: Platform,
    pub execution: ExecutionMode,
    pub handle: Option<String>,
    pub mode: crate::InstanceMode,
}

impl ParsedConfig<'_> {
    pub fn key<N: Node>(&self, normalized: Vec<String>) -> MiresidianResult<InstanceKey> {
        match &self.handle {
            Some(handle) if handle.is_empty() => Err(MiresidianError::InvalidArgument {
                field: "Handle".to_owned(),
                message: "Handle 为空，无法定位对象".to_owned(),
            }),
            Some(handle) => Ok(InstanceKey::Handle(handle.clone())),
            None => Ok(InstanceKey::Config(
                if self.mode == crate::InstanceMode::Multiple {
                    instance_key_indices::<N>()
                        .map(|index| normalized[index].clone())
                        .collect()
                } else {
                    normalized
                },
            )),
        }
    }
}

/// 只从业务 Config 选择实例键索引，与租约字段选择独立；框架附加字段不参与。
fn instance_key_indices<N: Node>() -> impl Iterator<Item = usize> {
    (0..N::descriptor().init_config.len())
        .filter(|index| N::instance_config_indices().is_none_or(|keys| keys.contains(index)))
}

impl<N: Node> NodeRuntime<N> {
    /// 提取稳定通道配置文本。业务参数缺参报错；该规则与宏生成参数解析保持一致，
    /// 因而参数名前缀、枚举显示值和实际值不会制造重复通道。
    pub(super) fn normalize_channel_config(
        &self,
        config: &[String],
        routing: bool,
    ) -> MiresidianResult<Vec<String>> {
        let policy = N::descriptor().instance;

        N::descriptor()
            .init_config
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let value = normalize_value_with_definition(
                    config,
                    index,
                    parameter.name,
                    parameter.definition,
                    parameter.default_value,
                )?;
                let is_identity = policy.mode == InstanceMode::Single
                    || (policy.binding == crate::InstanceBinding::Config
                        && instance_key_indices::<N>().any(|key| key == index));
                if !routing || is_identity {
                    catch_node(INIT_DEVICE_OPERATION, || {
                        N::normalize_config_value(index, &value)
                    })
                } else {
                    // 多实例查找由主键/Handle 决定，非主键值不替换对象保存的业务配置。
                    Ok(value)
                }
            })
            .collect()
    }

    /// 原始身份判空先于类型归一化；选中组合键任一项为空就跳过初始化。
    pub(super) fn has_business_key(&self, config: &[String]) -> bool {
        let mut indices = instance_key_indices::<N>().peekable();
        indices.peek().is_some()
            && indices.all(|index| {
                let parameter = &N::descriptor().init_config[index];
                config.get(index).is_some_and(|value| {
                    let value = value.trim();
                    let prefix = format!("{}:", parameter.name);
                    !value
                        .strip_prefix(&prefix)
                        .unwrap_or(value)
                        .trim()
                        .is_empty()
                })
            })
    }

    pub(super) fn parse_config<'a>(
        &self,
        config: &'a [String],
        explicit_adapter: Option<Platform>,
    ) -> MiresidianResult<ParsedConfig<'a>> {
        let descriptor = N::descriptor();
        let layout = ConfigLayout::new(
            descriptor.init_config.len(),
            descriptor.instance,
            descriptor.lease_activation.has_platform_selector(),
        );
        let node = &config[..config.len().min(layout.business)];
        let handle = layout.handle.map(|index| {
            let value = config
                .get(index)
                .map(|value| value.trim())
                .unwrap_or_default();
            value
                .strip_prefix("Handle:")
                .unwrap_or(value)
                .trim()
                .to_owned()
        });
        let access = descriptor.lease_activation.resolve_access(
            layout
                .lease
                .and_then(|index| config.get(index))
                .map(|value| {
                    value
                        .trim()
                        .strip_prefix("资源访问方式:")
                        .unwrap_or(value)
                        .trim()
                }),
        );
        let reserved_index = layout.platform;
        if config
            .get(reserved_index)
            .is_some_and(|value| self.is_platform_selector(value))
        {
            return Err(MiresidianError::InvalidArgument {
                field: "platform".into(),
                message: "平台由 DLL 请求的 platform 字段确定，不应在 Config 中重复选择".into(),
            });
        }
        let platform = explicit_adapter
            .or(self.active_platform())
            .unwrap_or(self.platforms.default_platform());
        if config.len() > reserved_index + usize::from(layout.execution) {
            return Err(MiresidianError::InvalidArgument {
                field: "Config".to_owned(),
                message: "配置参数数量超出初始化配置定义".to_owned(),
            });
        }
        let value = config.get(reserved_index).map(|value| {
            value
                .trim()
                .strip_prefix("共享执行方式:")
                .unwrap_or(value)
                .trim()
        });
        let execution = descriptor.instance.execution.resolve(value)?;
        Ok(ParsedConfig {
            node,
            access,
            platform,
            execution,
            handle,
            mode: descriptor.instance.mode,
        })
    }

    fn is_platform_selector(&self, value: &str) -> bool {
        let value = value.trim();
        value
            .split_once(":")
            .is_some_and(|(name, _)| name.trim() == PLATFORM_ADAPTER_NAME)
            || self.platforms.adapter_by_identifier(value).is_some()
    }

    pub(super) fn adapter_for(
        &self,
        invocation: &Invocation,
    ) -> MiresidianResult<std::sync::Arc<dyn PlatformAdapter>> {
        let _ = invocation;
        let platform = self.platforms.default_platform();
        self.platforms
            .adapter(platform)
            .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(platform.to_string()))
    }

    /// 本次实际适配器先将枚举输入转换为 codec 实际值，不保存适配器借用。
    pub(super) fn normalize_command(
        &self,
        adapter: &dyn PlatformAdapter,
        command: &mut crate::RuntimeInvocation,
    ) -> MiresidianResult<()> {
        let normalize = |values: &mut [String],
                         parameters: &[crate::ParameterDescriptor]|
         -> MiresidianResult<()> {
            for (value, parameter) in values.iter_mut().zip(parameters) {
                if !parameter.enum_options.is_empty() {
                    *value = adapter.normalize_parameter(parameter, value)?;
                }
            }
            Ok(())
        };
        let descriptor = N::descriptor();
        let inputs = |operation: &str| -> &[crate::ParameterDescriptor] {
            match operation {
                crate::INIT_DEVICE_OPERATION => descriptor.init_inputs,
                crate::GET_PORT_NO_OPERATION => descriptor.port_probe_inputs,
                _ => self
                    .operation_descriptor(operation)
                    .map_or(&[], |operation| operation.channel_inputs),
            }
        };
        match command {
            crate::RuntimeInvocation::Single(request) => {
                if matches!(
                    request.operation.as_str(),
                    crate::GET_FUNCTIONS_OPERATION
                        | crate::GET_INIT_CONFIG_OPERATION
                        | crate::GET_FRAMEWORK_VERSION_OPERATION
                        | crate::GET_PROJECT_VERSION_OPERATION
                ) {
                    return Ok(());
                }
                normalize(&mut request.config, descriptor.init_config)?;
                normalize(&mut request.input, inputs(&request.operation))?;
            }
            crate::RuntimeInvocation::MultiChannel(request) => {
                normalize(&mut request.config, descriptor.init_config)?;
                for input in request.slots.iter_mut().flatten() {
                    normalize(input, inputs(&request.operation))?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn operation_descriptor(
        &self,
        operation: &str,
    ) -> Option<&'static crate::OperationDescriptor> {
        N::descriptor()
            .operations
            .iter()
            .find(|descriptor| descriptor.name == operation)
    }
}
