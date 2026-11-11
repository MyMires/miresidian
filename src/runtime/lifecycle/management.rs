//! 生命周期事务串行协调对象构造和移除；独立实例的业务调用不持有管理锁。
use super::{Instance, NodeRuntime, catch_node, instance};
use crate::{CHANNEL_REUSED_MESSAGE, INIT_DEVICE_OPERATION};
use crate::{
    InstanceMode, InvocationOutput, MiresidianError, MiresidianResult, Node, PLATFORM_ADAPTER_NAME,
    Platform,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
impl<N: Node> NodeRuntime<N> {
    pub(super) fn channel_for_config(
        &self,
        config: &[String],
        platform: Option<Platform>,
    ) -> MiresidianResult<Arc<Instance<N>>> {
        let parsed = self.parse_config(config, platform)?;
        let key = parsed.key::<N>(self.normalize_channel_config(parsed.node, true)?)?;
        let management = self.management();
        if management.objects.is_empty() && !matches!(key, super::routing::InstanceKey::Handle(_)) {
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
    ) -> MiresidianResult<InvocationOutput> {
        self.check_accepting()?;
        let _lifecycle = self
            .core
            .lifecycle
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.check_accepting()?;
        crate::CallContext::current().check()?;
        let parsed = self.parse_config(config, adapter_identifier)?;
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
        let channel_config = self.normalize_channel_config(parsed.node, false)?;
        let key = parsed.key::<N>(channel_config.clone())?;
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
                        "Config"
                    }
                    .to_owned(),
                    message: "已有主键的业务 Config 不一致，不能重新绑定对象".to_owned(),
                });
            }
            if channel.access != parsed.access {
                return Err(MiresidianError::InvalidArgument {
                    field: "资源访问方式".to_owned(),
                    message: "已有对象的资源访问方式不能变更，请关闭后重新初始化".to_owned(),
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
    pub(super) fn close_device(
        &self,
        config: &[String],
        input: &[String],
        platform: Option<Platform>,
    ) -> MiresidianResult<InvocationOutput> {
        let _lifecycle = self
            .core
            .lifecycle
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::CallContext::current().check()?;
        let parsed = self.parse_config(config, platform)?;
        let key = parsed.key::<N>(self.normalize_channel_config(parsed.node, true)?)?;
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
    /// 暂停接收新请求，等待在途初始化与实例请求结束，再逐一关闭并析构对象。
    ///
    /// Direct 对象本地清理，Shared 引用释放代理；最后共享引用才清理所有者及租约。
    /// 所有并发 shutdown 调用结束后，运行时可再次初始化。关闭错误仍尽力清理其余对象。
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
    /// 异步关闭入口；生命周期清理交给独立执行器，避免阻塞调用方的异步线程。
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
        });
        Ok(InvocationOutput::new(match value {
            Ok(Some(port)) => vec!["1".into(), port],
            Ok(None) => vec!["0".into(), "未找到匹配设备".into()],
            Err(MiresidianError::NotSupported(reason)) => vec![
                "0".into(),
                if reason.trim().is_empty() {
                    "当前未实现端口发现能力".into()
                } else {
                    reason
                },
            ],
            Err(error) => return Err(error),
        }))
    }
}
