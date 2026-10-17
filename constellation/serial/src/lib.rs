//! 由 Miresidian 属性宏导出 DLL 入口的串口通信驱动。

use miresidian::{DeviceNode, DriverResult, InvocationOutput, NodeDescriptor, SerialNode};

/// 串口通信 DLL 的节点类型。
///
/// 当前先复用运行时中的串口通信实现；下一步会将具体串口节点本体迁入本 crate，
/// 使根运行时只保留通用传输抽象。
#[miresidian::node("串口通信")]
#[derive(Default)]
pub struct SerialDriver {
    inner: SerialNode,
}

#[miresidian::node_impl]
impl DeviceNode for SerialDriver {
    fn descriptor() -> &'static NodeDescriptor {
        SerialNode::descriptor()
    }

    fn init_device(&mut self, config: &[String]) -> DriverResult<InvocationOutput> {
        self.inner.init_device(config)
    }

    fn close_device(&mut self) -> DriverResult<()> {
        self.inner.close_device()
    }

    fn invoke(
        &mut self,
        operation: &str,
        config: &[String],
        input: &[String],
    ) -> DriverResult<InvocationOutput> {
        self.inner.invoke(operation, config, input)
    }

    fn get_port_no(&mut self) -> DriverResult<Option<String>> {
        self.inner.get_port_no()
    }
}
