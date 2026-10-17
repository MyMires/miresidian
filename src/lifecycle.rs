//! 设备节点的有状态生命周期与固定操作调度。

use crate::{
    DeviceNode, DriverError, DriverResult, Invocation, InvocationOutput, CLOSE_DEVICE_OPERATION,
    GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION,
    INIT_DEVICE_OPERATION,
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
    pub fn invoke(&mut self, invocation: &Invocation) -> DriverResult<InvocationOutput> {
        match invocation.operation.as_str() {
            GET_FUNCTIONS_OPERATION => Ok(InvocationOutput::new(
                N::descriptor().function_signatures(),
            )),
            GET_INIT_CONFIG_OPERATION => Ok(InvocationOutput::new(
                N::descriptor().init_config_definitions(),
            )),
            GET_PORT_NO_OPERATION => self.get_port_no(),
            INIT_DEVICE_OPERATION => self.init_device(&invocation.config),
            CLOSE_DEVICE_OPERATION => self.close_device(),
            operation => self.invoke_node(operation, &invocation.config, &invocation.input),
        }
    }

    /// 返回当前是否保留着已初始化节点。
    pub fn is_initialized(&self) -> bool {
        self.current.is_some()
    }

    fn init_device(&mut self, config: &[String]) -> DriverResult<InvocationOutput> {
        self.close_device()?;

        let mut candidate = N::default();
        match candidate.init_device(config) {
            Ok(output) => {
                self.current = Some(candidate);
                Ok(output)
            }
            Err(error) => {
                let _ = candidate.close_device();
                Err(error)
            }
        }
    }

    fn close_device(&mut self) -> DriverResult<InvocationOutput> {
        if let Some(mut current) = self.current.take() {
            current.close_device()?;
        }
        Ok(InvocationOutput::empty())
    }

    fn get_port_no(&mut self) -> DriverResult<InvocationOutput> {
        let mut probe = N::default();
        let port = probe.get_port_no();
        let _ = probe.close_device();

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
    ) -> DriverResult<InvocationOutput> {
        let current = self.current.as_mut().ok_or(DriverError::NotInitialized)?;
        current.invoke(operation, config, input)
    }
}

impl<N: DeviceNode> Default for NodeRuntime<N> {
    fn default() -> Self {
        Self::new()
    }
}
