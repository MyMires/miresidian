//! Concurrent runtime: management locks never cover user code or device waits.

use super::super::instances::InstanceStore;
use super::Instance;
use crate::{
    CallContext, DeviceNode, ExecutionLimits, InstanceMode, Invocation, InvocationOutput,
    MiresidianError, MiresidianResult, PLATFORM_ADAPTER_NAME, Platform, PlatformAdapter,
    PlatformRegistry, RuntimeInvocation, RuntimeResponse,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

pub(super) struct Management<N: DeviceNode> {
    pub(super) objects: InstanceStore<Arc<Instance<N>>>,
    pub(super) active_platform: Option<Platform>,
}
pub(super) struct RuntimeState<N: DeviceNode> {
    pub(super) management: Mutex<Management<N>>,
    // Serialize lifecycle transactions only; independent business continues.
    pub(super) lifecycle: Mutex<()>,
    pub(super) stopping: AtomicUsize,
    pub(super) pending: Arc<tokio::sync::Semaphore>,
    pub(super) running: Arc<tokio::sync::Semaphore>,
    pub(super) limits: ExecutionLimits,
}

/// One runtime manages independently executing instances. Clones share identity,
/// limits and lifecycle. A request (including a multi-channel batch) is exclusive
/// on its target instance. Use async methods inside an async executor.
pub struct NodeRuntime<N: DeviceNode> {
    pub(super) core: Arc<RuntimeState<N>>,
    pub(super) platforms: PlatformRegistry,
}
impl<N: DeviceNode> Clone for NodeRuntime<N> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            platforms: self.platforms.clone(),
        }
    }
}
impl<N: DeviceNode> NodeRuntime<N> {
    pub fn new() -> Self {
        Self::with_platforms(PlatformRegistry::default())
    }
    pub fn with_platforms(platforms: PlatformRegistry) -> Self {
        Self::with_limits(platforms, ExecutionLimits::default()).expect("valid default limits")
    }
    /// Configure positive runtime admission, execution and batch limits.
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
    pub fn invoke(&self, request: &Invocation) -> MiresidianResult<InvocationOutput> {
        super::super::execution::block_on(self.invoke_async(request.clone()))
    }
    /// Execute without blocking the caller's async thread. Dropping this future
    /// does not destroy an admitted request: the spawned execution owns its locks
    /// until completion. Use CallContext for cooperative cancellation.
    pub async fn invoke_async(&self, request: Invocation) -> MiresidianResult<InvocationOutput> {
        self.invoke_with_context(request, CallContext::default())
            .await
    }
    pub async fn invoke_with_context(
        &self,
        request: Invocation,
        context: CallContext,
    ) -> MiresidianResult<InvocationOutput> {
        let adapter = self.adapter_for(&request)?;
        self.invoke_mode(adapter, request, true, context).await
    }
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
    pub fn invoke_command(&self, command: RuntimeInvocation) -> MiresidianResult<RuntimeResponse> {
        super::super::execution::block_on(self.invoke_command_async(command))
    }
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
        // A detached caller cannot drop locks while synchronous code is running.
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
    /// Execute explicitly targeted requests concurrently, preserving result
    /// positions. Every item settles; an item error does not roll back others.
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
impl<N: DeviceNode> Default for NodeRuntime<N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<N: DeviceNode> Drop for RuntimeState<N> {
    fn drop(&mut self) {
        // Core destruction happens exactly once, including concurrent final
        // handle drops. No request can outlive its owned runtime core.
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
