//! 元数据共用布局下的 Config 解析与对象关联键。

use super::NodeRuntime;
use crate::runtime::instances::ConfigLayout;
use crate::{
    DeviceNode, ExecutionMode, INIT_DEVICE_OPERATION, Invocation, LeaseAccess, MiresidianError,
    MiresidianResult, PLATFORM_ADAPTER_NAME, Platform, PlatformAdapter,
};

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
    pub fn key(&self, normalized: Vec<String>) -> MiresidianResult<InstanceKey> {
        match &self.handle {
            Some(handle) if handle.is_empty() => Err(MiresidianError::InvalidArgument {
                field: "Handle".to_owned(),
                message: "Handle 为空，无法定位对象".to_owned(),
            }),
            Some(handle) => Ok(InstanceKey::Handle(handle.clone())),
            None => Ok(InstanceKey::Config(
                if self.mode == crate::InstanceMode::Multiple {
                    normalized.into_iter().take(1).collect()
                } else {
                    normalized
                },
            )),
        }
    }
}

impl<N: DeviceNode> NodeRuntime<N> {
    pub(super) fn parse_config<'a>(
        &self,
        config: &'a [String],
        platform_selector_enabled: bool,
        explicit_adapter: Option<Platform>,
    ) -> MiresidianResult<ParsedConfig<'a>> {
        let descriptor = N::descriptor();
        let layout = ConfigLayout::new(
            descriptor.init_config.len(),
            descriptor.instances,
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
        let mut reserved_index = layout.platform;
        let (platform, consumes_selector) = if platform_selector_enabled {
            self.platform_selection(config.get(reserved_index).map(String::as_str))?
        } else {
            if let Some(value) = config.get(reserved_index)
                && self.is_platform_selector(value)
            {
                return Err(MiresidianError::InvalidArgument {
                    field: PLATFORM_ADAPTER_NAME.to_owned(),
                    message: "invoke_with 已显式指定平台适配器，不应在 Config 中重复选择"
                        .to_owned(),
                });
            }
            (
                explicit_adapter
                    .or(self.active_platform)
                    .unwrap_or(self.platforms.default_platform()),
                false,
            )
        };
        reserved_index += usize::from(consumes_selector);
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
        let execution = descriptor.instances.execution.resolve(value)?;
        Ok(ParsedConfig {
            node,
            access,
            platform,
            execution,
            handle,
            mode: descriptor.instances.mode,
        })
    }

    fn is_platform_selector(&self, value: &str) -> bool {
        let value = value.trim();
        value
            .split_once(":")
            .is_some_and(|(name, _)| name.trim() == PLATFORM_ADAPTER_NAME)
            || self.platforms.adapter_by_identifier(value).is_some()
    }

    fn platform_selection(&self, value: Option<&str>) -> MiresidianResult<(Platform, bool)> {
        let Some(value) = value else {
            return Ok((self.platforms.default_platform(), false));
        };
        let value = value.trim();
        if let Some((name, selected)) = value.split_once(":")
            && name.trim() == PLATFORM_ADAPTER_NAME
        {
            let selected = selected.trim();
            return self
                .platforms
                .platform_by_identifier(selected)
                .map(|platform| (platform, true))
                .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(selected.to_owned()));
        }
        if let Some(platform) = self.platforms.platform_by_identifier(value) {
            return Ok((platform, true));
        }
        Ok((self.platforms.default_platform(), false))
    }

    pub(super) fn adapter_for(
        &self,
        invocation: &Invocation,
    ) -> MiresidianResult<std::sync::Arc<dyn PlatformAdapter>> {
        let platform = if invocation.operation == INIT_DEVICE_OPERATION {
            self.parse_config(&invocation.config, true, None)?.platform
        } else {
            self.active_platform
                .unwrap_or(self.platforms.default_platform())
        };
        self.platforms
            .adapter(platform)
            .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(platform.to_string()))
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
