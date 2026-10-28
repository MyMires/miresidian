//! 平台选择与框架保留初始化配置的解析。
//!
//! 本模块只负责将外部 Config 拆分为业务配置、资源访问方式、平台和共享调度方式；
//! 不创建、关闭或调用节点。

use super::NodeRuntime;
use crate::{
    DeviceNode, ExecutionMode, INIT_DEVICE_OPERATION, Invocation, LeaseAccess, MiresidianError,
    MiresidianResult, PLATFORM_ADAPTER_NAME, Platform, PlatformAdapter,
};

impl<N: DeviceNode> NodeRuntime<N> {
    pub(super) fn split_init_config<'a>(
        &self,
        config: &'a [String],
        platform_selector_enabled: bool,
        explicit_adapter: Option<Platform>,
    ) -> MiresidianResult<(&'a [String], LeaseAccess, Platform, ExecutionMode)> {
        let count = N::descriptor().init_config.len();
        let node_config = &config[..config.len().min(count)];
        let lease_selector = N::descriptor().lease_activation.has_platform_selector();
        let access = N::descriptor().lease_activation.resolve_access(
            lease_selector
                .then(|| config.get(count))
                .flatten()
                .map(String::as_str),
        );
        let mut reserved_index = count + usize::from(lease_selector);
        let (platform, consumes_platform_selector) = if platform_selector_enabled {
            self.platform_selection(config.get(reserved_index).map(String::as_str))?
        } else {
            if let Some(value) = config.get(reserved_index).map(String::as_str)
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
        reserved_index += usize::from(consumes_platform_selector);
        if config.len() > reserved_index + 1 {
            return Err(MiresidianError::InvalidArgument {
                field: "Config".to_owned(),
                message: "配置参数数量超出初始化配置定义".to_owned(),
            });
        }
        let execution =
            ExecutionMode::parse_platform_value(config.get(reserved_index).map(String::as_str));
        Ok((node_config, access, platform, execution))
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
                .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(selected.to_string()));
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
        let identifier = if invocation.operation == INIT_DEVICE_OPERATION {
            self.split_init_config(&invocation.config, true, None)?.2
        } else {
            self.active_platform
                .unwrap_or_else(|| self.platforms.default_platform())
        };
        self.platforms
            .adapter(identifier)
            .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(identifier.to_string()))
    }

    /// 返回操作的共享调度描述；未知操作返回 `None`。
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
