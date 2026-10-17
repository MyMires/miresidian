//! 具体设备节点实现的扩展点。

use crate::{DriverResult, InvocationOutput, NodeDescriptor};

/// 由 `#[miresidian::node_impl]` 为具体驱动生成的运行时适配器。
///
/// 驱动作者不应手写此 trait。宏会从 `operation`、`config`、`input` 与 `output`
/// 标注生成实现、强类型转换和描述元数据。
pub trait DeviceNode: Default + Send + 'static {
    /// 返回此节点类型的编译期元数据。
    fn descriptor() -> &'static NodeDescriptor;

    /// 使用初始化配置建立设备连接。
    fn init_device(&mut self, config: &[String]) -> DriverResult<InvocationOutput>;

    /// 释放节点持有的全部资源。
    fn close_device(&mut self, input: &[String]) -> DriverResult<()>;

    /// 在成功初始化后运行节点自定义操作。
    fn invoke(
        &mut self,
        operation: &str,
        config: &[String],
        input: &[String],
    ) -> DriverResult<InvocationOutput>;

    /// 使用临时节点实例探测端口。
    ///
    /// 返回 `Ok(None)` 表示旧版 `GetPortNO` 结果 `["0", ""]`。
    fn get_port_no(&mut self, _input: &[String]) -> DriverResult<Option<String>> {
        Ok(None)
    }
}
