//! Lifecycle transactions serialize construction/removal, while business on
//! independent instances proceeds without the management lock.
use super::{Instance, NodeRuntime, catch_node, instance};
use crate::{CHANNEL_REUSED_MESSAGE, INIT_DEVICE_OPERATION, normalize_value_with_definition};
use crate::{
    DeviceNode, InstanceMode, InvocationOutput, MiresidianError, MiresidianResult,
    PLATFORM_ADAPTER_NAME, Platform,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
impl<N: DeviceNode> NodeRuntime<N> {
    pub(super) fn channel_for_config(
        &self,
        config: &[String],
        selector: bool,
        platform: Option<Platform>,
    ) -> MiresidianResult<Arc<Instance<N>>> {
        let parsed = self.parse_config(config, selector, platform)?;
        let key = parsed.key(self.normalize_channel_config(parsed.node)?)?;
        let management = self.management();
        if management.objects.is_empty() {
            return Err(MiresidianError::NotInitialized);
        }
        management
            .objects
            .iter()
            .find(|v| v.key == key)
            .cloned()
            .ok_or_else(|| MiresidianError::UnknownChannel(key.description()))
    }
    pub(super) fn init_device(
        &self,
        config: &[String],
        input: &[String],
        adapter_identifier: Option<Platform>,
        platform_selector_enabled: bool,
    ) -> MiresidianResult<InvocationOutput> {
        self.check_accepting()?;
        let _lifecycle = self
            .core
            .lifecycle
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.check_accepting()?;
        crate::CallContext::current().check()?;
        let parsed = self.parse_config(config, platform_selector_enabled, adapter_identifier)?;
        if parsed.handle.as_deref() == Some("")
            || (N::descriptor().instances.mode == InstanceMode::Multiple
                && parsed.handle.is_none()
                && !self.has_business_key(parsed.node))
        {
            return Ok(InvocationOutput::empty());
        }
        let platform = parsed.platform;
        if let Some(active) = self.active_platform()
            && active != platform
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
        let channel_config = self.normalize_channel_config(parsed.node)?;
        let key = parsed.key(channel_config.clone())?;
        let existing = {
            self.management()
                .objects
                .iter()
                .find(|channel| channel.key == key)
                .cloned()
        };
        if let Some(channel) = existing {
            if channel.config != channel_config {
                return Err(MiresidianError::InvalidArgument {
                    field: if parsed.handle.is_some() {
                        "Handle"
                    } else {
                        "Config[0]"
                    }
                    .to_owned(),
                    message: "已有主键的业务 Config 不一致，不能重新绑定对象".to_owned(),
                });
            }
            if channel.execution != parsed.execution {
                return Err(MiresidianError::InvalidArgument {
                    field: "共享执行方式".to_owned(),
                    message: format!(
                        "当前对象已使用 {}，不能改为 {}",
                        channel.execution.definition(),
                        parsed.execution.definition()
                    ),
                });
            }
            channel.ensure_active()?;
            return Ok(InvocationOutput::new(vec![
                CHANNEL_REUSED_MESSAGE.to_owned(),
            ]));
        }
        // 仅预解析参数。预校验失败不影响旧对象；构造仍在旧资源彻底释放后执行。
        instance::catch_node(INIT_DEVICE_OPERATION, || {
            N::validate_init(parsed.node, input)
        })?;
        if N::descriptor().instances.mode == InstanceMode::Single
            && !self.management().objects.is_empty()
        {
            self.cleanup_all()?;
        }
        crate::CallContext::current().check()?;
        let (channel, output) = Instance::initialize(
            key,
            channel_config,
            parsed.node,
            input,
            parsed.access,
            parsed.execution,
            self.core.limits.max_pending_per_instance,
        )?;
        if let Err(error) = crate::CallContext::current().check() {
            let _ = channel.close(&[]);
            return Err(error);
        }
        if self.check_accepting().is_err() {
            let _ = channel.close(&[]);
            return Err(MiresidianError::Failure("初始化被全局关闭阻止登记".into()));
        }
        let channel = Arc::new(channel);
        let mut management = self.management();
        if let Err(channel) = management.objects.push(channel) {
            drop(management);
            let _ = channel.close(&[]);
            return Err(MiresidianError::Internal("实例容器已满".into()));
        }
        management.active_platform = Some(platform);
        Ok(output)
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

    /// `Config[0]` 是业务主键；判空先于默认值，其他业务或框架字段均不能替代它。
    fn has_business_key(&self, config: &[String]) -> bool {
        N::descriptor()
            .init_config
            .first()
            .zip(config.first())
            .is_some_and(|(parameter, value)| {
                let value = value.trim();
                let prefix = format!("{}:", parameter.name);
                !value
                    .strip_prefix(&prefix)
                    .unwrap_or(value)
                    .trim()
                    .is_empty()
            })
    }
    pub(super) fn close_device(
        &self,
        config: &[String],
        input: &[String],
        selector: bool,
        platform: Option<Platform>,
    ) -> MiresidianResult<InvocationOutput> {
        let _lifecycle = self
            .core
            .lifecycle
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::CallContext::current().check()?;
        let parsed = self.parse_config(config, selector, platform)?;
        let key = parsed.key(self.normalize_channel_config(parsed.node)?)?;
        let channel = {
            let management = self.management();
            management
                .objects
                .iter()
                .find(|v| v.key == key)
                .cloned()
                .ok_or_else(|| MiresidianError::UnknownChannel(key.description()))?
        };
        channel.mark_closing();
        let result = channel.close(input);
        let mut management = self.management();
        let index = management
            .objects
            .iter()
            .position(|v| v.key == key)
            .expect("lifecycle protects identity");
        management.objects.remove(index);
        if management.objects.is_empty() {
            management.active_platform = None;
        }
        result.map(|()| InvocationOutput::empty())
    }
    /// Reject new requests, wait for in-flight initialization and instance
    /// requests, then close/drop every object before releasing its lease.
    /// The runtime accepts initialization again when all shutdown callers finish.
    pub fn shutdown(&self) -> MiresidianResult<()> {
        self.core.stopping.fetch_add(1, Ordering::AcqRel);
        struct Reset<'a>(&'a AtomicUsize);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::AcqRel);
            }
        }
        let _reset = Reset(&self.core.stopping);
        let _lifecycle = self
            .core
            .lifecycle
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.cleanup_all()
    }
    /// Async shutdown bridge; lifecycle cleanup runs off the caller executor.
    pub async fn shutdown_async(&self) -> MiresidianResult<()> {
        let runtime = self.clone();
        super::super::execution::lifecycle_executor()?
            .spawn_blocking(move || runtime.shutdown())
            .await
            .map_err(|e| MiresidianError::Internal(e.to_string()))?
    }
    fn cleanup_all(&self) -> MiresidianResult<()> {
        let objects = {
            let management = self.management();
            management.objects.iter().cloned().collect::<Vec<_>>()
        };
        for channel in &objects {
            channel.mark_closing();
        }
        let mut failures = Vec::new();
        for channel in objects.iter().rev() {
            if let Err(error) = channel.close(&[]) {
                failures.push(format!(
                    "通道 [{}] 关闭失败: {error}",
                    channel.key.description()
                ));
            }
        }
        let mut management = self.management();
        while management.objects.pop().is_some() {}
        management.active_platform = None;
        if failures.is_empty() {
            Ok(())
        } else {
            Err(MiresidianError::Failure(format!(
                "关闭全部通道时发生清理失败: {}",
                failures.join("；")
            )))
        }
    }
    pub(super) fn get_port_no(
        &self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        let value = catch_node(crate::GET_PORT_NO_OPERATION, || {
            N::get_port_no(config, input)
        })?;
        Ok(InvocationOutput::new(match value {
            Some(port) => vec!["1".into(), port],
            None => vec!["0".into(), String::new()],
        }))
    }
}
