//! 设备节点的有状态生命周期与固定操作调度。
//!
//! 运行时按 Config 键管理多个通道对象：`InitDevice` 以 Config 精确匹配复用或创建对象，
//! 其余方法以 Config 定位唯一对象执行。共享调用由 `shared` 模块在全部已创建通道上批量执行。

use std::sync::{Arc, Mutex};

use crate::{
    CHANNEL_REUSED_MESSAGE, CLOSE_DEVICE_OPERATION, DeviceNode, GET_FRAMEWORK_VERSION_OPERATION,
    GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION,
    GET_PROJECT_VERSION_OPERATION, INIT_DEVICE_OPERATION, Invocation, InvocationOutput,
    MIRESIDIAN_VERSION, MiresidianError, MiresidianResult,
};

/// 一个按 Config 键登记的通道对象。
pub(super) struct Channel<N> {
    /// 创建该对象时的完整 Config 数组；作为定位键原样保留。
    pub(super) config: Vec<String>,
    /// 通道对象本体；共享调用在多线程间通过互斥锁复用。
    pub(super) node: Arc<Mutex<N>>,
}

/// 持有全部已初始化通道对象，并执行平台生命周期规则。
pub struct NodeRuntime<N: DeviceNode> {
    channels: Vec<Channel<N>>,
}

impl<N: DeviceNode> NodeRuntime<N> {
    /// 创建尚未初始化节点的运行时。
    pub fn new() -> Self {
        Self { channels: vec![] }
    }

    /// 处理固定操作或节点自定义操作的调用。
    pub fn invoke(&mut self, invocation: &Invocation) -> MiresidianResult<InvocationOutput> {
        match invocation.operation.as_str() {
            GET_FUNCTIONS_OPERATION => {
                Ok(InvocationOutput::new(N::descriptor().function_signatures()))
            }
            GET_FRAMEWORK_VERSION_OPERATION => {
                Ok(InvocationOutput::new(vec![MIRESIDIAN_VERSION.to_owned()]))
            }
            GET_PROJECT_VERSION_OPERATION => {
                Ok(InvocationOutput::new(vec![N::project_version().to_owned()]))
            }
            GET_INIT_CONFIG_OPERATION => Ok(InvocationOutput::new(
                N::descriptor().init_config_definitions(),
            )),
            GET_PORT_NO_OPERATION => self.get_port_no(&invocation.config, &invocation.input),
            INIT_DEVICE_OPERATION => self.init_device(&invocation.config, &invocation.input),
            CLOSE_DEVICE_OPERATION => self.close_device(&invocation.config, &invocation.input),
            operation => self.invoke_node(operation, &invocation.config, &invocation.input),
        }
    }

    /// 返回当前是否保留着至少一个已初始化通道对象。
    pub fn is_initialized(&self) -> bool {
        !self.channels.is_empty()
    }

    /// 以 Config 精确匹配定位通道对象。
    fn find_channel(&self, config: &[String]) -> Option<&Channel<N>> {
        self.channels
            .iter()
            .find(|channel| channel.config.as_slice() == config)
    }

    fn init_device(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        if self.find_channel(config).is_some() {
            return Ok(InvocationOutput::new(vec![
                CHANNEL_REUSED_MESSAGE.to_owned(),
            ]));
        }

        // 仅在初始化成功后登记对象；失败的候选对象仍尝试关闭，以释放部分建立的资源。
        let mut candidate = N::default();
        match candidate.init_device(config, input) {
            Ok(output) => {
                self.channels.push(Channel {
                    config: config.to_vec(),
                    node: Arc::new(Mutex::new(candidate)),
                });
                Ok(output)
            }
            Err(error) => {
                let _ = candidate.close_device(&[], &[]);
                Err(error)
            }
        }
    }

    fn close_device(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        let index = self
            .channels
            .iter()
            .position(|channel| channel.config.as_slice() == config)
            .ok_or_else(|| MiresidianError::UnknownChannel(config.join(",")))?;
        // 关闭失败时保留登记项，使调用方能够修正条件后重试关闭。
        {
            let mut node = self.channels[index]
                .node
                .lock()
                .map_err(|_| MiresidianError::Failure("节点状态锁已损坏".to_owned()))?;
            node.close_device(config, input)?;
        }
        self.channels.remove(index);
        Ok(InvocationOutput::empty())
    }

    fn get_port_no(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        let mut probe = N::default();
        let port = probe.get_port_no(config, input);
        let _ = probe.close_device(&[], &[]);

        match port? {
            Some(port) => Ok(InvocationOutput::new(vec!["1".to_owned(), port])),
            None => Ok(InvocationOutput::new(vec!["0".to_owned(), String::new()])),
        }
    }

    fn invoke_node(
        &mut self,
        operation: &str,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        // 共享调用刻意忽略本次 Config：每个已登记通道使用其自身的创建 Config。
        if super::shared::is_shared_invocation(input) {
            if self.channels.is_empty() {
                return Err(MiresidianError::NotInitialized);
            }
            let descriptor = self
                .operation_descriptor(operation)
                .ok_or_else(|| MiresidianError::InvalidOperation(operation.to_owned()))?;
            return super::shared::invoke_shared(
                &self.channels,
                operation,
                &input[1..],
                descriptor,
            );
        }

        if self.channels.is_empty() {
            return Err(MiresidianError::NotInitialized);
        }
        let channel = self
            .find_channel(config)
            .ok_or_else(|| MiresidianError::UnknownChannel(config.join(",")))?;

        let mut node = channel
            .node
            .lock()
            .map_err(|_| MiresidianError::Failure("节点状态锁已损坏".to_owned()))?;
        node.invoke(operation, config, input)
    }

    /// 返回操作的共享调度描述；未知操作返回 `None`。
    fn operation_descriptor(&self, operation: &str) -> Option<&'static crate::OperationDescriptor> {
        N::descriptor()
            .operations
            .iter()
            .find(|descriptor| descriptor.name == operation)
    }
}

impl<N: DeviceNode> Default for NodeRuntime<N> {
    fn default() -> Self {
        Self::new()
    }
}
