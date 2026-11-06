//! 现有 lease 标注的资源对象代理。所有者持有 N，后续 DLL 只持有 IPC 引用。
mod wire;
use super::{catch_node, cleanup_node};
use crate::{
    CallContext, DeviceNode, INIT_DEVICE_OPERATION, InvocationOutput, MiresidianError,
    MiresidianResult,
    lease::{
        LeaseError, LeaseRequest,
        shared::{ResourceHandler, SharedResource},
    },
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wire::{Close, Request};

const PROTOCOL: &str = "miresidian-node/1";

pub(super) struct SharedNode {
    resource: SharedResource,
}
impl SharedNode {
    pub(super) fn acquire<N: DeviceNode>(
        request: LeaseRequest,
        config: &[String],
        raw_config: &[String],
        input: &[String],
    ) -> MiresidianResult<(Self, InvocationOutput)> {
        let contract = contract::<N>();
        let owner = Owner::<N> {
            node: None,
            fault: None,
            output: None,
            contract: contract.clone(),
            config: config.to_vec(),
            raw_config: raw_config.to_vec(),
            input: input.to_vec(),
        };
        let resource = SharedResource::acquire(
            request.resource().clone(),
            request.configuration().clone(),
            PROTOCOL,
            move || Ok(owner),
        )
        .map_err(|error| MiresidianError::Failure(error.to_string()))?;
        let mut shared = Self { resource };
        let initialized = shared.request(&Request::Initialize {
            contract,
            config: config.to_vec(),
            input: input.to_vec(),
        });
        match initialized {
            Ok(output) => Ok((shared, InvocationOutput::new(output))),
            Err(error) => {
                let _ = shared.close(&[]);
                Err(error)
            }
        }
    }
    fn request(&self, request: &Request) -> MiresidianResult<Vec<String>> {
        let context = CallContext::current();
        context.check()?;
        let bytes = serde_json::to_vec(request)
            .map_err(|error| MiresidianError::Internal(error.to_string()))?;
        if bytes.len() > crate::lease::shared::MAX_TRANSACTION_BYTES {
            return Err(MiresidianError::InvalidArgument {
                field: "Input".into(),
                message: "节点代理请求超过 16 MiB".into(),
            });
        }
        context.check()?;
        let response = match self
            .resource
            .transact_with_timeout(&bytes, context.remaining())
        {
            Ok(response) => response,
            Err(error) => {
                context.check()?;
                return Err(ipc_error(error));
            }
        };
        wire::decode(&response)
    }
    pub(super) fn validate(&self) -> MiresidianResult<()> {
        self.request(&Request::Validate).map(|_| ())
    }
    pub(super) fn invoke(
        &self,
        operation: &str,
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        let deadline_ms = CallContext::current().remaining().map(|duration| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .saturating_add(duration)
                .as_millis()
                .min(u64::MAX as u128) as u64
        });
        self.request(&Request::Invoke {
            operation: operation.into(),
            input: input.to_vec(),
            deadline_ms,
        })
        .map(InvocationOutput::new)
    }
    pub(super) fn close(&mut self, input: &[String]) -> MiresidianResult<()> {
        let request = serde_json::to_vec(&Close {
            input: input.to_vec(),
        })
        .map_err(|error| MiresidianError::Internal(error.to_string()))?;
        // 关闭必须完成清理；取消标记不能阻止释放引用。
        let response = self
            .resource
            .close_with(&request, None)
            .map_err(ipc_error)?;
        if response.is_empty() {
            Ok(())
        } else {
            wire::decode(&response).map(|_| ())
        }
    }
}
fn ipc_error(error: LeaseError) -> MiresidianError {
    MiresidianError::LeaseInvalidated(error.to_string())
}

fn contract<N: DeviceNode>() -> Vec<String> {
    let descriptor = N::descriptor();
    let mut operations = descriptor
        .operations
        .iter()
        .map(|operation| operation.signature())
        .collect::<Vec<_>>();
    operations.sort();
    // Node 名称标识共同业务契约；文件名、Rust 类型路径、项目版本和 Handle 不参与。
    vec![
        descriptor.name.to_owned(),
        format!("{:?}", descriptor.init_config),
        format!("{:?}", descriptor.init_inputs),
        format!("{:?}", descriptor.init_outputs),
        operations.join("\n"),
    ]
}

struct Owner<N: DeviceNode> {
    node: Option<N>,
    fault: Option<MiresidianError>,
    output: Option<Vec<String>>,
    contract: Vec<String>,
    config: Vec<String>,
    raw_config: Vec<String>,
    input: Vec<String>,
}
impl<N: DeviceNode> Owner<N> {
    fn execute(&mut self, request: Request) -> MiresidianResult<Vec<String>> {
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        match request {
            Request::Initialize {
                contract,
                config,
                input,
            } => {
                if contract != self.contract {
                    return Err(MiresidianError::InvalidArgument {
                        field: "lease".into(),
                        message: "同一资源的节点契约不兼容".into(),
                    });
                }
                if config != self.config || input != self.input {
                    return Err(MiresidianError::InvalidArgument { field: "Config/Input".into(), message: "共享节点的完整初始化参数必须一致；独立业务状态请使用通信层 SharedResource".into() });
                }
                if let Some(output) = &self.output {
                    return Ok(output.clone());
                }
                match catch_node(INIT_DEVICE_OPERATION, || {
                    N::init_device(&self.raw_config, &self.input)
                }) {
                    Ok((node, output)) => {
                        self.node = Some(node);
                        let output = output.into_values();
                        self.output = Some(output.clone());
                        Ok(output)
                    }
                    Err(error) => {
                        self.fault = Some(error.clone());
                        Err(error)
                    }
                }
            }
            Request::Validate => self
                .node
                .as_ref()
                .map(|_| Vec::new())
                .ok_or(MiresidianError::NotInitialized),
            Request::Invoke {
                operation,
                input,
                deadline_ms,
            } => {
                let node = self.node.as_mut().ok_or(MiresidianError::NotInitialized)?;
                let remaining = deadline_ms.map(|deadline| {
                    Duration::from_millis(deadline).saturating_sub(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default(),
                    )
                });
                let context = CallContext::new(remaining);
                context.check()?;
                let result = if N::is_async_operation(&operation) {
                    use futures_util::FutureExt;
                    crate::runtime::execution::block_on(context.scope(async {
                        std::panic::AssertUnwindSafe(node.invoke_async(
                            &operation,
                            &self.config,
                            &input,
                        ))
                        .catch_unwind()
                        .await
                        .unwrap_or_else(|_| {
                            Err(MiresidianError::NodePanicked {
                                operation: operation.clone(),
                            })
                        })
                    }))
                } else {
                    context.blocking(|| {
                        catch_node(&operation, || node.invoke(&operation, &self.config, &input))
                    })
                };
                if let Err(error @ MiresidianError::NodePanicked { .. }) = &result {
                    self.fault = Some(error.clone());
                }
                result.map(InvocationOutput::into_values)
            }
        }
    }
    fn cleanup(&mut self, input: &[String]) -> MiresidianResult<Vec<String>> {
        self.node
            .take()
            .map(|node| cleanup_node(node, &self.config, input))
            .transpose()?;
        Ok(Vec::new())
    }
}
impl<N: DeviceNode> ResourceHandler for Owner<N> {
    fn transact(&mut self, bytes: &[u8]) -> Result<Vec<u8>, LeaseError> {
        let request = serde_json::from_slice(bytes)
            .map_err(|error| LeaseError::LocalIpc(error.to_string()))?;
        wire::encode(self.execute(request))
    }
    fn close(&mut self, bytes: &[u8]) -> Result<Vec<u8>, LeaseError> {
        let input = if bytes.is_empty() {
            Vec::new()
        } else {
            serde_json::from_slice::<Close>(bytes)
                .map_err(|error| LeaseError::LocalIpc(error.to_string()))?
                .input
        };
        wire::encode(self.cleanup(&input))
    }
}
impl<N: DeviceNode> Drop for Owner<N> {
    fn drop(&mut self) {
        let _ = self.cleanup(&[]);
    }
}
