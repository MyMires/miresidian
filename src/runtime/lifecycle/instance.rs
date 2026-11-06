//! 单通道的异常隔离、租约校验与资源清理。
//!
//! 节点的可变状态与故障标记由同一把锁保护。panic 在持锁的调用内部捕获，
//! 故障对象仅允许清理；清理完成后才能由运行时重新初始化。

use super::routing::InstanceKey;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(feature = "lease")]
use super::shared::SharedNode;
use crate::{
    CLOSE_DEVICE_OPERATION, DeviceNode, INIT_DEVICE_OPERATION, InvocationOutput, LeaseAccess,
    MiresidianError, MiresidianResult,
};

pub(in crate::runtime) struct Instance<N> {
    pub(super) key: InstanceKey,
    pub(in crate::runtime) config: Vec<String>,
    pub(in crate::runtime) execution: crate::ExecutionMode,
    pub(super) access: LeaseAccess,
    state: Arc<tokio::sync::Mutex<Option<InstanceState<N>>>>,
    pub(super) request: tokio::sync::Mutex<()>,
    pub(super) pending: Arc<tokio::sync::Semaphore>,
    closing: AtomicBool,
}

enum NodeAccess<N> {
    Direct(N),
    #[cfg(feature = "lease")]
    Shared(SharedNode),
}

impl<N> NodeAccess<N> {
    fn direct_mut(&mut self) -> &mut N {
        match self {
            Self::Direct(node) => node,
            #[cfg(feature = "lease")]
            Self::Shared(_) => unreachable!("shared nodes execute on the owner"),
        }
    }
}

struct InstanceState<N> {
    node: NodeAccess<N>,
    fault: Option<MiresidianError>,
}

impl<N: DeviceNode> Instance<N> {
    pub(super) fn initialize(
        key: InstanceKey,
        config: Vec<String>,
        node_config: &[String],
        input: &[String],
        access: LeaseAccess,
        execution: crate::ExecutionMode,
        pending_limit: usize,
    ) -> MiresidianResult<(Self, InvocationOutput)> {
        #[cfg(feature = "lease")]
        if matches!(access, LeaseAccess::Lease) {
            let request = catch_node(INIT_DEVICE_OPERATION, || N::lease_request(node_config))?
                .ok_or_else(|| {
                    MiresidianError::NotSupported("Lease 模式必须提供资源租约请求".into())
                })?;
            let (shared, output) = SharedNode::acquire::<N>(request, &config, node_config, input)?;
            return Ok((
                Self::new(
                    key,
                    config,
                    access,
                    execution,
                    pending_limit,
                    NodeAccess::Shared(shared),
                ),
                output,
            ));
        }
        let (node, output) =
            catch_node(INIT_DEVICE_OPERATION, || N::init_device(node_config, input))?;
        Ok((
            Self::new(
                key,
                config,
                access,
                execution,
                pending_limit,
                NodeAccess::Direct(node),
            ),
            output,
        ))
    }

    fn new(
        key: InstanceKey,
        config: Vec<String>,
        access: LeaseAccess,
        execution: crate::ExecutionMode,
        pending_limit: usize,
        node: NodeAccess<N>,
    ) -> Self {
        Self {
            key,
            config,
            access,
            execution,
            request: tokio::sync::Mutex::new(()),
            pending: Arc::new(tokio::sync::Semaphore::new(pending_limit)),
            closing: AtomicBool::new(false),
            state: Arc::new(tokio::sync::Mutex::new(Some(InstanceState {
                node,
                fault: None,
            }))),
        }
    }

    pub(super) fn mark_closing(&self) {
        self.closing.store(true, Ordering::Release);
    }

    pub(super) fn check_open(&self) -> MiresidianResult<()> {
        if self.closing.load(Ordering::Acquire) {
            Err(MiresidianError::Failure("实例正在关闭".into()))
        } else {
            Ok(())
        }
    }

    pub(in crate::runtime) fn ensure_active(&self) -> MiresidianResult<()> {
        self.check_open()?;
        self.state
            .blocking_lock()
            .as_mut()
            .ok_or(MiresidianError::NotInitialized)?
            .ensure_active()
    }

    pub(in crate::runtime) async fn invoke(
        &self,
        operation: &str,
        input: &[String],
        context: &crate::CallContext,
    ) -> MiresidianResult<InvocationOutput> {
        use futures_util::FutureExt;
        context.check()?;
        let state = self.state.clone().lock_owned().await;
        let operation = operation.to_owned();
        let input = input.to_vec();
        let config = self.config.clone();
        let call_context = context.clone();
        let native_operation = operation.clone();
        let native_input = input.clone();
        // Lease IPC and synchronous user code never block an async worker.
        let native = N::is_async_operation(&operation)
            && state
                .as_ref()
                .is_some_and(|inner| matches!(inner.node, NodeAccess::Direct(_)));
        let (mut state, result) = tokio::task::spawn_blocking(move || {
            let mut state = state;
            let result = call_context.blocking(|| {
                call_context.check()?;
                let inner = state.as_mut().ok_or(MiresidianError::NotInitialized)?;
                inner.ensure_active()?;
                if native {
                    return Ok(None);
                }
                catch_node(&operation, || match &mut inner.node {
                    NodeAccess::Direct(node) => node.invoke(&operation, &config, &input),
                    #[cfg(feature = "lease")]
                    NodeAccess::Shared(shared) => shared.invoke(&operation, &input),
                })
                .map(Some)
            });
            if let Err(error @ MiresidianError::NodePanicked { .. }) = &result
                && let Some(inner) = state.as_mut()
            {
                inner.fault = Some(error.clone());
            }
            (state, result)
        })
        .await
        .map_err(|e| MiresidianError::Internal(e.to_string()))?;
        match result? {
            Some(output) => Ok(output),
            None => {
                context.check()?;
                let inner = state.as_mut().ok_or(MiresidianError::NotInitialized)?;
                let node = inner.node.direct_mut();
                let result = context
                    .scope(
                        AssertUnwindSafe(node.invoke_async(
                            &native_operation,
                            &self.config,
                            &native_input,
                        ))
                        .catch_unwind(),
                    )
                    .await;
                let result = result.unwrap_or({
                    Err(MiresidianError::NodePanicked {
                        operation: native_operation,
                    })
                });
                if let Err(error @ MiresidianError::NodePanicked { .. }) = &result {
                    inner.fault = Some(error.clone());
                }
                result
            }
        }
    }

    pub(in crate::runtime) fn close(&self, input: &[String]) -> MiresidianResult<()> {
        self.mark_closing();
        let _request = self.request.blocking_lock();
        let Some(state) = self.state.blocking_lock().take() else {
            return Ok(());
        };
        match state.node {
            NodeAccess::Direct(node) => cleanup_node(node, &self.config, input),
            #[cfg(feature = "lease")]
            NodeAccess::Shared(mut shared) => shared.close(input),
        }
    }
}

impl<N> InstanceState<N> {
    fn ensure_active(&mut self) -> MiresidianResult<()> {
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        #[cfg(feature = "lease")]
        if let NodeAccess::Shared(shared) = &self.node {
            // 远端故障/会话失效由所有者和代理保存；本次排队取消不能污染引用。
            shared.validate()?;
        }
        Ok(())
    }
}

pub(in crate::runtime) fn catch_node<T>(
    operation: &str,
    call: impl FnOnce() -> MiresidianResult<T>,
) -> MiresidianResult<T> {
    catch_unwind(AssertUnwindSafe(call)).unwrap_or_else(|_| {
        Err(MiresidianError::NodePanicked {
            operation: operation.to_owned(),
        })
    })
}

pub(in crate::runtime) fn cleanup_node<N: DeviceNode>(
    mut node: N,
    config: &[String],
    input: &[String],
) -> MiresidianResult<()> {
    let closed = catch_node(CLOSE_DEVICE_OPERATION, || node.close_device(config, input));
    // 分别捕获 close 与 Drop 的异常，避免 close panic 导致析构在展开期间再次 panic。
    let dropped = catch_node("释放节点", || {
        drop(node);
        Ok(())
    });
    match (closed, dropped) {
        (Err(error @ MiresidianError::NodePanicked { .. }), _)
        | (_, Err(error @ MiresidianError::NodePanicked { .. })) => Err(error),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}
