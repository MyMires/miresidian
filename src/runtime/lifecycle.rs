//! 设备节点的有状态生命周期与固定操作调度。

use crate::{
    CLOSE_DEVICE_OPERATION, DeviceNode, FRAMEWORK_VERSION, GET_FRAMEWORK_VERSION_OPERATION,
    GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION,
    GET_PROJECT_VERSION_OPERATION, INIT_DEVICE_OPERATION, Invocation, InvocationOutput,
    MiresidianError, MiresidianResult,
};

/// 持有当前已初始化节点，并执行平台生命周期规则。
pub struct NodeRuntime<N: DeviceNode> {
    current: Option<N>,
}

impl<N: DeviceNode> NodeRuntime<N> {
    /// 创建尚未初始化节点的运行时。
    pub fn new() -> Self {
        Self { current: None }
    }

    /// 处理固定操作或节点自定义操作的调用。
    pub fn invoke(&mut self, invocation: &Invocation) -> MiresidianResult<InvocationOutput> {
        match invocation.operation.as_str() {
            GET_FUNCTIONS_OPERATION => {
                Ok(InvocationOutput::new(N::descriptor().function_signatures()))
            }
            GET_FRAMEWORK_VERSION_OPERATION => {
                Ok(InvocationOutput::new(vec![FRAMEWORK_VERSION.to_owned()]))
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

    /// 返回当前是否保留着已初始化节点。
    pub fn is_initialized(&self) -> bool {
        self.current.is_some()
    }

    fn init_device(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        self.close_device(&[], &[])?;

        let mut candidate = N::default();
        match candidate.init_device(config, input) {
            Ok(output) => {
                self.current = Some(candidate);
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
        if let Some(mut current) = self.current.take() {
            current.close_device(config, input)?;
        }
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
        let current = self
            .current
            .as_mut()
            .ok_or(MiresidianError::NotInitialized)?;
        current.invoke(operation, config, input)
    }
}

impl<N: DeviceNode> Default for NodeRuntime<N> {
    fn default() -> Self {
        Self::new()
    }
}
