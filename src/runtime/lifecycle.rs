//! 设备节点的有状态生命周期与固定操作调度。
//!
//! 运行时按规范化后的业务 Config 键管理多个通道对象：`InitDevice` 以等价 Config 复用或
//! 创建对象，其余方法以同一规则定位唯一对象。共享调用由 `shared` 模块在全部已创建通道上
//! 批量执行；平台选择与框架保留 Config 的解析收敛到 `routing` 子模块。

mod channel;
mod routing;

use crate::{
    CHANNEL_REUSED_MESSAGE, CLOSE_ALL_DEVICES_OPERATION, CLOSE_DEVICE_OPERATION, DeviceNode,
    ExecutionMode, GET_FRAMEWORK_VERSION_OPERATION, GET_FUNCTIONS_OPERATION,
    GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION, GET_PROJECT_VERSION_OPERATION,
    INIT_DEVICE_OPERATION, Invocation, InvocationOutput, MIRESIDIAN_VERSION, MiresidianError,
    MiresidianResult, PLATFORM_ADAPTER_NAME, Platform, PlatformAdapter, PlatformRegistry,
    RuntimeInvocation, RuntimeResponse, SharedInvocation, SharedResponse,
    normalize_value_with_definition,
};

pub(super) use channel::Channel;

/// 持有全部已初始化通道对象，并执行平台生命周期规则。
pub struct NodeRuntime<N: DeviceNode> {
    channels: Vec<Channel<N>>,
    /// 首个成功初始化的通道决定当前驱动运行时的共享调度方式。
    shared_execution: Option<ExecutionMode>,
    /// 可由 `InitDevice` 的保留 Config 选择的平台适配器。
    platforms: PlatformRegistry,
    /// 运行时持有通道期间固定使用的平台；释放全部通道后允许重新选择。
    active_platform: Option<Platform>,
}

impl<N: DeviceNode> NodeRuntime<N> {
    /// 创建尚未初始化节点的运行时，默认仅注册 TB LV Test Platform 适配器。
    pub fn new() -> Self {
        Self::with_platforms(PlatformRegistry::default())
    }

    /// 创建使用指定平台注册表的运行时。
    pub fn with_platforms(platforms: PlatformRegistry) -> Self {
        Self {
            channels: vec![],
            shared_execution: None,
            platforms,
            active_platform: None,
        }
    }

    /// 使用 `InitDevice` 的 `平台适配` 配置所选择的适配器处理调用。
    pub fn invoke(&mut self, invocation: &Invocation) -> MiresidianResult<InvocationOutput> {
        let adapter = self.adapter_for(invocation)?;
        self.invoke_with_mode(adapter.as_ref(), invocation.clone(), true)
    }

    /// 按平台枚举从注册表选择适配器处理调用。
    pub fn invoke_with_platform(
        &mut self,
        platform: Platform,
        request: Invocation,
    ) -> MiresidianResult<InvocationOutput> {
        let adapter =
            self.platforms
                .adapter(platform)
                .ok_or_else(|| MiresidianError::InvalidArgument {
                    field: PLATFORM_ADAPTER_NAME.to_owned(),
                    message: format!("未注册的平台适配器 {platform}"),
                })?;
        self.invoke_with(adapter.as_ref(), request)
    }

    /// 使用指定平台适配器处理调用。直接调用此方法时，`InitDevice` 的平台选择器仍会
    /// 校验该适配器的稳定标识，避免以错误协议初始化运行时。
    pub fn invoke_with<A: PlatformAdapter + ?Sized>(
        &mut self,
        adapter: &A,
        request: Invocation,
    ) -> MiresidianResult<InvocationOutput> {
        self.invoke_with_mode(adapter, request, false)
    }

    fn invoke_with_mode<A: PlatformAdapter + ?Sized>(
        &mut self,
        adapter: &A,
        request: Invocation,
        platform_selector_enabled: bool,
    ) -> MiresidianResult<InvocationOutput> {
        if let Some(active) = &self.active_platform
            && active != &adapter.platform()
        {
            return Err(MiresidianError::InvalidArgument {
                field: PLATFORM_ADAPTER_NAME.to_owned(),
                message: format!(
                    "当前驱动运行时已使用 {active}，不能改用 {}",
                    adapter.platform()
                ),
            });
        }
        let command = adapter.decode(request)?;
        let response =
            self.invoke_command_from(Some(adapter.platform()), platform_selector_enabled, command)?;
        adapter.encode(response)
    }

    /// 执行平台无关的运行时命令。
    pub fn invoke_command(
        &mut self,
        command: RuntimeInvocation,
    ) -> MiresidianResult<RuntimeResponse> {
        self.invoke_command_from(None, true, command)
    }

    fn invoke_command_from(
        &mut self,
        adapter_identifier: Option<Platform>,
        platform_selector_enabled: bool,
        command: RuntimeInvocation,
    ) -> MiresidianResult<RuntimeResponse> {
        match command {
            RuntimeInvocation::Single(invocation) => self
                .invoke_single(&invocation, adapter_identifier, platform_selector_enabled)
                .map(RuntimeResponse::Output),
            RuntimeInvocation::Shared(invocation) => {
                self.invoke_shared(invocation).map(RuntimeResponse::Shared)
            }
        }
    }

    /// 处理固定操作或单通道节点操作。
    fn invoke_single(
        &mut self,
        invocation: &Invocation,
        adapter_identifier: Option<Platform>,
        platform_selector_enabled: bool,
    ) -> MiresidianResult<InvocationOutput> {
        match invocation.operation.as_str() {
            GET_FUNCTIONS_OPERATION => Ok(InvocationOutput::new(if platform_selector_enabled {
                N::descriptor()
                    .function_signatures_with_platform(&self.platforms.selector_definition())
            } else {
                N::descriptor().function_signatures()
            })),
            GET_FRAMEWORK_VERSION_OPERATION => {
                Ok(InvocationOutput::new(vec![MIRESIDIAN_VERSION.to_owned()]))
            }
            GET_PROJECT_VERSION_OPERATION => {
                Ok(InvocationOutput::new(vec![N::project_version().to_owned()]))
            }
            GET_INIT_CONFIG_OPERATION => Ok(InvocationOutput::new(if platform_selector_enabled {
                N::descriptor()
                    .init_config_definitions_with_platform(&self.platforms.selector_definition())
            } else {
                N::descriptor().init_config_definitions()
            })),
            GET_PORT_NO_OPERATION => self.get_port_no(&invocation.config, &invocation.input),
            INIT_DEVICE_OPERATION => self.init_device(
                &invocation.config,
                &invocation.input,
                adapter_identifier,
                platform_selector_enabled,
            ),
            CLOSE_DEVICE_OPERATION => self.close_device(
                &invocation.config,
                &invocation.input,
                platform_selector_enabled,
                adapter_identifier,
            ),
            CLOSE_ALL_DEVICES_OPERATION => self.close_all(&invocation.config, &invocation.input),
            operation => self.invoke_node(
                operation,
                &invocation.config,
                &invocation.input,
                platform_selector_enabled,
                adapter_identifier,
            ),
        }
    }

    /// 返回当前是否保留着至少一个已初始化通道对象。
    pub fn is_initialized(&self) -> bool {
        !self.channels.is_empty()
    }

    /// 关闭并释放全部已初始化通道。
    ///
    /// 无论单个节点关闭或租约释放是否失败，运行时都会移除全部通道并重置平台与共享调度
    /// 状态，避免已关闭或半关闭对象继续参与后续调用。失败会在全部清理尝试完成后汇总返回。
    pub fn shutdown(&mut self) -> MiresidianResult<()> {
        let mut failures = Vec::new();

        while let Some(channel) = self.channels.pop() {
            let channel_key = channel.config.join(",");
            if let Err(error) = channel.close(&[]) {
                failures.push(format!("通道 [{channel_key}] 关闭失败: {error}"));
            }
        }

        self.shared_execution = None;
        self.active_platform = None;

        if failures.is_empty() {
            Ok(())
        } else {
            Err(MiresidianError::Failure(format!(
                "关闭全部通道时发生清理失败: {}",
                failures.join("；")
            )))
        }
    }

    /// 以规范化后的业务 Config 定位通道对象。
    fn find_channel(&self, config: &[String]) -> Option<&Channel<N>> {
        self.channels
            .iter()
            .find(|channel| channel.config.as_slice() == config)
    }

    fn init_device(
        &mut self,
        config: &[String],
        input: &[String],
        adapter_identifier: Option<Platform>,
        platform_selector_enabled: bool,
    ) -> MiresidianResult<InvocationOutput> {
        let (node_config, access, platform, execution) =
            self.split_init_config(config, platform_selector_enabled, adapter_identifier)?;
        if let Some(active) = &self.active_platform
            && active != &platform
        {
            return Err(MiresidianError::InvalidArgument {
                field: PLATFORM_ADAPTER_NAME.to_owned(),
                message: format!("当前驱动运行时已使用 {active}，不能改为 {platform}"),
            });
        }
        if let Some(adapter_identifier) = adapter_identifier
            && adapter_identifier != platform
        {
            return Err(MiresidianError::InvalidArgument {
                field: PLATFORM_ADAPTER_NAME.to_owned(),
                message: format!("配置选择 {platform}，但本次调用使用 {adapter_identifier}"),
            });
        }
        let channel_config = self.normalize_channel_config(node_config)?;
        if let Some(active) = self.shared_execution
            && active != execution
        {
            return Err(MiresidianError::InvalidArgument {
                field: "共享执行方式".to_owned(),
                message: format!(
                    "当前驱动运行时已使用 {}，不能改为 {}",
                    active.definition(),
                    execution.definition()
                ),
            });
        }
        if let Some(channel) = self.find_channel(&channel_config) {
            channel.ensure_active()?;
            return Ok(InvocationOutput::new(vec![
                CHANNEL_REUSED_MESSAGE.to_owned(),
            ]));
        }

        let (channel, output) = Channel::initialize(channel_config, node_config, input, access)?;
        self.channels.push(channel);
        self.shared_execution = Some(execution);
        self.active_platform = Some(platform);
        Ok(output)
    }

    fn close_device(
        &mut self,
        config: &[String],
        input: &[String],
        platform_selector_enabled: bool,
        adapter_identifier: Option<Platform>,
    ) -> MiresidianResult<InvocationOutput> {
        let config = self.normalize_channel_config(
            self.split_init_config(config, platform_selector_enabled, adapter_identifier)?
                .0,
        )?;
        let index = self
            .channels
            .iter()
            .position(|channel| channel.config == config)
            .ok_or_else(|| MiresidianError::UnknownChannel(config.join(",")))?;
        // 关闭尝试后始终移除对象，避免半关闭或故障节点再次参与业务调用。
        let result = self.channels.remove(index).close(input);
        if self.channels.is_empty() {
            self.shared_execution = None;
            self.active_platform = None;
        }
        result.map(|()| InvocationOutput::empty())
    }

    fn close_all(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        if !config.is_empty() || !input.is_empty() {
            return Err(MiresidianError::InvalidArgument {
                field: CLOSE_ALL_DEVICES_OPERATION.to_owned(),
                message: "不接受 Config 或 Input 参数".to_owned(),
            });
        }
        self.shutdown()?;
        Ok(InvocationOutput::empty())
    }

    fn get_port_no(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        let mut probe = channel::catch_node(GET_PORT_NO_OPERATION, || Ok(N::default()))?;
        let port = channel::catch_node(GET_PORT_NO_OPERATION, || probe.get_port_no(config, input));
        let cleanup = channel::cleanup_node(probe, config, &[]);
        let port = port?;
        cleanup?;

        match port {
            Some(port) => Ok(InvocationOutput::new(vec!["1".to_owned(), port])),
            None => Ok(InvocationOutput::new(vec!["0".to_owned(), String::new()])),
        }
    }

    /// 执行平台已经结构化的共享调用。
    fn invoke_shared(&mut self, invocation: SharedInvocation) -> MiresidianResult<SharedResponse> {
        if self.channels.is_empty() {
            return Err(MiresidianError::NotInitialized);
        }
        let descriptor = self
            .operation_descriptor(&invocation.operation)
            .ok_or_else(|| MiresidianError::InvalidOperation(invocation.operation.clone()))?;
        super::shared::invoke_shared(
            &self.channels,
            &invocation.operation,
            &invocation.slots,
            descriptor,
            self.shared_execution.unwrap_or(ExecutionMode::Sequential),
        )
    }

    fn invoke_node(
        &mut self,
        operation: &str,
        config: &[String],
        input: &[String],
        platform_selector_enabled: bool,
        adapter_identifier: Option<Platform>,
    ) -> MiresidianResult<InvocationOutput> {
        if self.channels.is_empty() {
            return Err(MiresidianError::NotInitialized);
        }
        let config = self.normalize_channel_config(
            self.split_init_config(config, platform_selector_enabled, adapter_identifier)?
                .0,
        )?;
        let channel = self
            .find_channel(&config)
            .ok_or_else(|| MiresidianError::UnknownChannel(config.join(",")))?;

        channel.invoke(operation, input)
    }

    /// 生成稳定通道键并补齐有效的默认值。该规则与宏生成参数解析保持一致，
    /// 因而参数名前缀、枚举显示值和实际值不会制造重复通道。
    fn normalize_channel_config(&self, config: &[String]) -> MiresidianResult<Vec<String>> {
        N::descriptor()
            .init_config
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                normalize_value_with_definition(
                    config,
                    index,
                    parameter.name,
                    parameter.definition,
                    parameter.default_value,
                )
            })
            .collect()
    }
}

impl<N: DeviceNode> Default for NodeRuntime<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: DeviceNode> Drop for NodeRuntime<N> {
    fn drop(&mut self) {
        // Drop 不能传播生命周期错误，也不能因节点实现 panic 而在析构期间二次 panic。
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.shutdown()));
    }
}
