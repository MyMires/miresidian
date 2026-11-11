//! 并发节点运行时；管理锁仅保护登记与查找，不覆盖业务代码或设备等待。

use super::super::instances::InstanceStore;
use super::Instance;
use crate::{
    CallContext, ExecutionLimits, InstanceMode, Invocation, InvocationOutput, MiresidianError,
    MiresidianResult, Node, PLATFORM_ADAPTER_NAME, Platform, PlatformAdapter, PlatformRegistry,
    RuntimeInvocation, RuntimeResponse,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

pub(super) struct Management<N: Node> {
    pub(super) objects: InstanceStore<Arc<Instance<N>>>,
    pub(super) active_platform: Option<Platform>,
}
pub(super) struct RuntimeState<N: Node> {
    pub(super) management: Mutex<Management<N>>,
    // 只串行化构造和移除事务；独立实例的业务仍可继续。
    pub(super) lifecycle: Mutex<()>,
    pub(super) stopping: AtomicUsize,
    pub(super) pending: Arc<tokio::sync::Semaphore>,
    pub(super) running: Arc<tokio::sync::Semaphore>,
    pub(super) limits: ExecutionLimits,
}

/// 管理节点实例、平台绑定与请求预算的运行时句柄。
///
/// Clone 共用登记表、容量和生命周期；同一实例的完整请求（含多通道批次）互斥，
/// 不同实例可并行。Direct 持有本地对象，自动 Lease 持有唯一节点所有者的共享引用。
/// 异步调用方使用 async 方法；同步桥在单线程异步执行器中返回能力错误。
pub struct NodeRuntime<N: Node> {
    pub(super) core: Arc<RuntimeState<N>>,
    pub(super) platforms: PlatformRegistry,
}
impl<N: Node> Clone for NodeRuntime<N> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            platforms: self.platforms.clone(),
        }
    }
}
impl<N: Node> NodeRuntime<N> {
    /// 创建默认平台注册表及容量的运行时；尚不构造设备，初始化须显式调用 InitDevice。
    pub fn new() -> Self {
        Self::with_platforms(PlatformRegistry::default())
    }
    /// 使用给定平台注册表和默认容量创建运行时；平台切换受首次初始化绑定约束。
    pub fn with_platforms(platforms: PlatformRegistry) -> Self {
        Self::with_limits(platforms, ExecutionLimits::default()).expect("valid default limits")
    }
    /// 用给定平台注册表和容量创建运行时，尚不构造业务节点。
    ///
    /// limits 各项须在 1..=65536，否则返回 InvalidArgument；Clone 共享这些限额。
    /// 初始实例策略来自节点描述，不会因开启 feature 自动改成多实例。
    pub fn with_limits(
        platforms: PlatformRegistry,
        limits: ExecutionLimits,
    ) -> MiresidianResult<Self> {
        let limits = limits.validate()?;
        let mut objects = InstanceStore::default();
        if cfg!(feature = "instances") || N::descriptor().instances.mode == InstanceMode::Single {
            objects.set_mode(N::descriptor().instances.mode)?;
        }
        Ok(Self {
            platforms,
            core: Arc::new(RuntimeState {
                management: Mutex::new(Management {
                    objects,
                    active_platform: None,
                }),
                lifecycle: Mutex::new(()),
                stopping: AtomicUsize::new(0),
                pending: Arc::new(tokio::sync::Semaphore::new(limits.max_pending)),
                running: Arc::new(tokio::sync::Semaphore::new(limits.max_running)),
                limits,
            }),
        })
    }
    pub(super) fn management(&self) -> std::sync::MutexGuard<'_, Management<N>> {
        self.core
            .management
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
    pub(super) fn active_platform(&self) -> Option<Platform> {
        self.management().active_platform
    }
    pub(super) fn check_accepting(&self) -> MiresidianResult<()> {
        if self.core.stopping.load(Ordering::Acquire) > 0 {
            Err(MiresidianError::Failure("运行时正在关闭".into()))
        } else {
            Ok(())
        }
    }
    /// 同步调用默认平台路径，返回编码后的业务输出或参数、状态及业务错误。
    /// InitDevice 可按保留 Config 选择平台；异步线程应使用 invoke_async，避免同步等待。
    pub fn invoke(&self, request: &Invocation) -> MiresidianResult<InvocationOutput> {
        super::super::execution::block_on(self.invoke_async(request.clone()))
    }
    /// 异步执行一次请求，返回平台编码后的业务输出或对应参数、状态及业务错误。
    ///
    /// 丢弃等待 Future 不会终止已被调度的请求；内部任务持有实例执行权直到安全结束。
    /// 需要协作取消或截止时间时，使用 invoke_with_context 传入 CallContext。
    pub async fn invoke_async(&self, request: Invocation) -> MiresidianResult<InvocationOutput> {
        self.invoke_with_context(request, CallContext::default())
            .await
    }
    /// 用给定上下文调用默认平台路径；上下文预算包含排队，取消不回滚已执行业务。
    /// 返回与 invoke_async 相同的输出/错误，并可能在安全边界报告取消或截止时间错误。
    pub async fn invoke_with_context(
        &self,
        request: Invocation,
        context: CallContext,
    ) -> MiresidianResult<InvocationOutput> {
        let adapter = self.adapter_for(&request)?;
        self.invoke_mode(adapter, request, true, context).await
    }
    /// 同步按已注册的平台编号调用；未知编号、与当前平台冲突或重复平台 Config 返回错误。
    /// 此路径不发布也不接受初始化的“平台适配”选择项；其他业务布局与默认入口相同。
    pub fn invoke_with_platform(
        &self,
        platform: Platform,
        request: Invocation,
    ) -> MiresidianResult<InvocationOutput> {
        super::super::execution::block_on(self.invoke_platform_async(
            platform,
            request,
            CallContext::default(),
        ))
    }
    /// invoke_with_platform 的异步形式，使用给定上下文；输出和平台错误规则保持一致。
    /// 丢弃等待不终止已接收的请求，取消需通过 context 协作完成。
    pub async fn invoke_platform_async(
        &self,
        platform: Platform,
        request: Invocation,
        context: CallContext,
    ) -> MiresidianResult<InvocationOutput> {
        let adapter = self
            .platforms
            .adapter(platform)
            .ok_or_else(|| MiresidianError::UnknownPlatformAdapter(platform.to_string()))?;
        self.invoke_mode(adapter, request, false, context).await
    }
    /// 使用调用方提供的适配器同步解码请求、执行命令并编码响应。
    /// 与当前平台绑定冲突、适配器转换错误或业务错误原样返回；初始化不接受平台选择器。
    pub fn invoke_with<A: PlatformAdapter + ?Sized>(
        &self,
        adapter: &A,
        request: Invocation,
    ) -> MiresidianResult<InvocationOutput> {
        self.check_platform(adapter.platform())?;
        let command = adapter.decode(request)?;
        let response = super::super::execution::block_on(self.command_async(
            command,
            Some(adapter.platform()),
            false,
            CallContext::default(),
        ))?;
        adapter.encode(response)
    }
    async fn invoke_mode(
        &self,
        adapter: Arc<dyn PlatformAdapter>,
        request: Invocation,
        selector: bool,
        context: CallContext,
    ) -> MiresidianResult<InvocationOutput> {
        self.check_platform(adapter.platform())?;
        let command = adapter.decode(request)?;
        let response = self
            .command_async(command, Some(adapter.platform()), selector, context)
            .await?;
        adapter.encode(response)
    }
    pub(super) fn check_platform(&self, platform: Platform) -> MiresidianResult<()> {
        if self
            .active_platform()
            .is_some_and(|active| active != platform)
        {
            return Err(MiresidianError::InvalidArgument {
                field: PLATFORM_ADAPTER_NAME.into(),
                message: format!("当前运行时不能改用 {platform}"),
            });
        }
        Ok(())
    }
    /// 同步执行已结构化的命令，跳过平台 decode/encode，返回结构化响应或运行时错误。
    /// 通道身份、容量、初始化和实例门禁仍由运行时校验。
    pub fn invoke_command(&self, command: RuntimeInvocation) -> MiresidianResult<RuntimeResponse> {
        super::super::execution::block_on(self.invoke_command_async(command))
    }
    /// invoke_command 的异步形式；不额外编码平台输出，仍等待完整请求安全结束。
    pub async fn invoke_command_async(
        &self,
        command: RuntimeInvocation,
    ) -> MiresidianResult<RuntimeResponse> {
        self.command_async(command, None, true, CallContext::default())
            .await
    }
    async fn command_async(
        &self,
        command: RuntimeInvocation,
        platform: Option<Platform>,
        selector: bool,
        context: CallContext,
    ) -> MiresidianResult<RuntimeResponse> {
        N::validate_definition()?;
        context.check()?;
        if let RuntimeInvocation::Single(request) = &command
            && let Some(output) = self.metadata(&request.operation, selector)
        {
            return Ok(RuntimeResponse::Output(output));
        }
        self.check_accepting()?;
        let admission = self
            .core
            .pending
            .clone()
            .try_acquire_owned()
            .map_err(|_| MiresidianError::Failure("运行时请求队列已满".into()))?;
        let activity = super::super::execution::Activity::new();
        let runtime = self.clone();
        // 请求由内部任务持有；调用方停止等待时，同步业务仍保留执行权直至实际返回。
        super::super::execution::executor()?
            .spawn(async move {
                use futures_util::FutureExt;
                let _activity = activity;
                let _admission = admission;
                let result = std::panic::AssertUnwindSafe(
                    runtime.execute(command, platform, selector, context),
                )
                .catch_unwind()
                .await;
                let value = match result {
                    Ok(value) => value,
                    Err(_) => {
                        let cleanup = runtime.clone();
                        let _ = super::super::execution::lifecycle_executor()?
                            .spawn_blocking(move || cleanup.shutdown())
                            .await;
                        Err(MiresidianError::Internal(
                            "运行时请求异常，已清理实例".into(),
                        ))
                    }
                };
                drop(runtime);
                value
            })
            .await
            .map_err(|e| MiresidianError::Internal(e.to_string()))?
    }
    async fn execute(
        &self,
        command: RuntimeInvocation,
        platform: Option<Platform>,
        selector: bool,
        context: CallContext,
    ) -> MiresidianResult<RuntimeResponse> {
        use crate::{
            CLOSE_ALL_DEVICES_OPERATION, CLOSE_DEVICE_OPERATION, GET_PORT_NO_OPERATION,
            INIT_DEVICE_OPERATION,
        };
        if let RuntimeInvocation::Single(request) = &command
            && matches!(
                request.operation.as_str(),
                INIT_DEVICE_OPERATION
                    | CLOSE_DEVICE_OPERATION
                    | CLOSE_ALL_DEVICES_OPERATION
                    | GET_PORT_NO_OPERATION
            )
        {
            let runtime = self.clone();
            let request = request.clone();
            let blocking_context = context.clone();
            return super::super::execution::lifecycle_executor()?
                .spawn_blocking(move || {
                    blocking_context.blocking(|| {
                        blocking_context.check()?;
                        blocking_context.mark_started();
                        let result = match request.operation.as_str() {
                            INIT_DEVICE_OPERATION => runtime.init_device(
                                &request.config,
                                &request.input,
                                platform,
                                selector,
                            ),
                            CLOSE_DEVICE_OPERATION => runtime.close_device(
                                &request.config,
                                &request.input,
                                selector,
                                platform,
                            ),
                            CLOSE_ALL_DEVICES_OPERATION => {
                                if !request.config.is_empty() || !request.input.is_empty() {
                                    return Err(MiresidianError::InvalidArgument {
                                        field: CLOSE_ALL_DEVICES_OPERATION.into(),
                                        message: "不接受 Config 或 Input 参数".into(),
                                    });
                                }
                                runtime.shutdown().map(|()| InvocationOutput::empty())
                            }
                            _ => runtime.get_port_no(&request.config, &request.input),
                        };
                        result.map(RuntimeResponse::Output)
                    })
                })
                .await
                .map_err(|e| MiresidianError::Internal(e.to_string()))?;
        }
        let (operation, config) = match &command {
            RuntimeInvocation::Single(r) => (&r.operation, &r.config),
            RuntimeInvocation::MultiChannel(r) => (&r.operation, &r.config),
        };
        let operation = operation.clone();
        let instance = self.channel_for_config(config, selector, platform)?;
        let _pending = instance
            .pending
            .clone()
            .try_acquire_owned()
            .map_err(|_| MiresidianError::Failure("实例请求队列已满".into()))?;
        let _request = context.wait(instance.request.lock()).await?;
        self.check_accepting()?;
        if let Some(platform) = platform {
            self.check_platform(platform)?;
        }
        instance.check_open()?;
        let _running = context
            .wait(self.core.running.clone().acquire_owned())
            .await?
            .map_err(|e| MiresidianError::Internal(e.to_string()))?;
        context.check()?;
        context.mark_started();
        match command {
            RuntimeInvocation::Single(request) => instance
                .invoke(&request.operation, &request.input, &context)
                .await
                .map(RuntimeResponse::Output),
            RuntimeInvocation::MultiChannel(request) => {
                if !N::descriptor().instances.enabled() {
                    return Err(MiresidianError::NotSupported(
                        "未声明 instances 多通道能力".into(),
                    ));
                }
                if request.slots.len() > self.core.limits.max_batch {
                    return Err(MiresidianError::Failure("批次超过容量限制".into()));
                }
                let descriptor = self
                    .operation_descriptor(&operation)
                    .ok_or_else(|| MiresidianError::InvalidOperation(operation.clone()))?;
                #[cfg(feature = "instances")]
                {
                    super::super::multi_channel::invoke_multi_channel(
                        &instance,
                        &request.operation,
                        &request.slots,
                        descriptor,
                        instance.execution,
                        &context,
                    )
                    .await
                    .map(RuntimeResponse::MultiChannel)
                }
                #[cfg(not(feature = "instances"))]
                {
                    let _ = descriptor;
                    Err(MiresidianError::NotSupported("未启用 instances".into()))
                }
            }
        }
    }
    /// 并发执行各自携带目标 Config 的请求，按原位置返回每项成功或错误。
    ///
    /// 超过 max_batch 时在开始前返回整体错误；接收后等待全部项结束，单项失败不回滚其他项。
    /// 各项仍遵守实例门禁，因此同实例互斥，不同实例才可实际并行。
    pub async fn invoke_batch_async(
        &self,
        requests: Vec<Invocation>,
    ) -> MiresidianResult<Vec<MiresidianResult<InvocationOutput>>> {
        if requests.len() > self.core.limits.max_batch {
            return Err(MiresidianError::Failure("批次超过容量限制".into()));
        }
        Ok(futures_util::future::join_all(
            requests
                .into_iter()
                .map(|request| self.invoke_async(request)),
        )
        .await)
    }
    /// invoke_batch_async 的同步桥；外层错误表示批次拒绝，每项结果仍按请求位置独立返回。
    pub fn invoke_batch(
        &self,
        requests: Vec<Invocation>,
    ) -> MiresidianResult<Vec<MiresidianResult<InvocationOutput>>> {
        super::super::execution::block_on(self.invoke_batch_async(requests))
    }

    fn metadata(&self, operation: &str, selector: bool) -> Option<InvocationOutput> {
        use crate::*;
        Some(InvocationOutput::new(match operation {
            GET_FUNCTIONS_OPERATION => {
                if selector {
                    N::descriptor()
                        .function_signatures_with_platform(&self.platforms.selector_definition())
                } else {
                    N::descriptor().function_signatures()
                }
            }
            GET_INIT_CONFIG_OPERATION => {
                if selector {
                    N::descriptor().init_config_definitions_with_platform(
                        &self.platforms.selector_definition(),
                    )
                } else {
                    N::descriptor().init_config_definitions()
                }
            }
            GET_FRAMEWORK_VERSION_OPERATION => vec![MIRESIDIAN_VERSION.into()],
            GET_PROJECT_VERSION_OPERATION => vec![N::project_version().into()],
            _ => return None,
        }))
    }
    pub(crate) fn batch_limit(&self) -> usize {
        self.core.limits.max_batch
    }
    pub fn is_initialized(&self) -> bool {
        !self.management().objects.is_empty()
    }
}
impl<N: Node> Default for NodeRuntime<N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<N: Node> Drop for RuntimeState<N> {
    fn drop(&mut self) {
        // 共享核心只在最后一个 Arc 释放时析构；请求持有核心，因此不会在业务运行中清理对象。
        // 即使多个句柄并发释放，以下实例清理也只执行一次。
        let management = self.management.get_mut().unwrap_or_else(|e| e.into_inner());
        let mut objects = Vec::new();
        while let Some(instance) = management.objects.pop() {
            objects.push(instance);
        }
        let cleanup = move || {
            for instance in objects {
                let _ = instance.close(&[]);
            }
        };
        if tokio::runtime::Handle::try_current().is_ok() {
            let _ = std::thread::spawn(cleanup).join();
        } else {
            cleanup();
        }
    }
}
