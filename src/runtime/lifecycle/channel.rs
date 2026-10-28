//! 单通道的异常隔离、租约校验与资源清理。
//!
//! 节点的可变状态与故障标记由同一把锁保护。panic 在持锁的调用内部捕获，
//! 故障对象仅允许清理；清理完成后才能由运行时重新初始化。

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Mutex,
};

#[cfg(feature = "lease")]
use crate::lease::local_ipc::LocalLease;
use crate::{
    CLOSE_DEVICE_OPERATION, DeviceNode, INIT_DEVICE_OPERATION, InvocationOutput, LeaseAccess,
    MiresidianError, MiresidianResult,
};

pub(in crate::runtime) struct Channel<N> {
    pub(in crate::runtime) config: Vec<String>,
    state: Mutex<ChannelState<N>>,
}

struct ChannelState<N> {
    node: N,
    fault: Option<MiresidianError>,
    #[cfg(feature = "lease")]
    lease: Option<LocalLease>,
}

impl<N: DeviceNode> Channel<N> {
    pub(in crate::runtime) fn initialize(
        config: Vec<String>,
        node_config: &[String],
        input: &[String],
        access: LeaseAccess,
    ) -> MiresidianResult<(Self, InvocationOutput)> {
        let mut node = None;
        #[cfg(feature = "lease")]
        let mut lease = None;
        #[cfg(not(feature = "lease"))]
        let _ = access;
        let initialized = (|| {
            #[cfg(feature = "lease")]
            if matches!(access, LeaseAccess::Lease) {
                lease = catch_node(INIT_DEVICE_OPERATION, || N::lease_request(node_config))?
                    .map(|request| {
                        LocalLease::acquire(
                            request.resource().clone(),
                            request.configuration().clone(),
                        )
                        .map_err(|error| MiresidianError::Failure(error.to_string()))
                    })
                    .transpose()?;
            }
            let (created, output) =
                catch_node(INIT_DEVICE_OPERATION, || N::init_device(node_config, input))?;
            node = Some(created);
            #[cfg(feature = "lease")]
            if let Some(lease) = &lease {
                lease
                    .validate()
                    .map_err(|error| MiresidianError::Failure(error.to_string()))?;
            }
            Ok(output)
        })();
        match initialized {
            Ok(output) => Ok((
                Self {
                    config,
                    state: Mutex::new(ChannelState {
                        node: node.expect("successful initialization returns a node"),
                        fault: None,
                        #[cfg(feature = "lease")]
                        lease,
                    }),
                },
                output,
            )),
            Err(error) => {
                // 节点先清理并析构，之后才释放租约所持有的配置锁。
                if let Some(node) = node {
                    let _ = cleanup_node(node, node_config, &[]);
                }
                #[cfg(feature = "lease")]
                if let Some(mut lease) = lease {
                    let _ = lease.release();
                }
                Err(error)
            }
        }
    }

    pub(in crate::runtime) fn ensure_active(&self) -> MiresidianResult<()> {
        self.state
            .lock()
            .map_err(|_| MiresidianError::NodeStatePoisoned)?
            .ensure_active()
    }

    pub(in crate::runtime) fn invoke(
        &self,
        operation: &str,
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| MiresidianError::NodeStatePoisoned)?;
        state.ensure_active()?;
        let result = catch_node(operation, || {
            state.node.invoke(operation, &self.config, input)
        });
        if let Err(error @ MiresidianError::NodePanicked { .. }) = &result {
            state.fault = Some(error.clone());
        }
        result
    }

    pub(in crate::runtime) fn close(self, input: &[String]) -> MiresidianResult<()> {
        // 污染对象只用于尽力关闭和析构，绝不恢复业务调用。
        let state = self
            .state
            .into_inner()
            .unwrap_or_else(|error| error.into_inner());
        let result = cleanup_node(state.node, &self.config, input);
        #[cfg(feature = "lease")]
        let release = state
            .lease
            .map(|mut lease| lease.release())
            .transpose()
            .map_err(|error| MiresidianError::Failure(format!("租约释放失败: {error}")));
        #[cfg(not(feature = "lease"))]
        let release: MiresidianResult<()> = Ok(());
        result?;
        release.map(|_| ())
    }
}

impl<N> ChannelState<N> {
    fn ensure_active(&mut self) -> MiresidianResult<()> {
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        #[cfg(feature = "lease")]
        if let Some(lease) = &self.lease
            && let Err(error) = lease.validate()
        {
            let error = MiresidianError::LeaseInvalidated(error.to_string());
            self.fault = Some(error.clone());
            return Err(error);
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
