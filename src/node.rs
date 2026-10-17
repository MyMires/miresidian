//! 具体设备节点实现的扩展点。

use crate::{DriverResult, InvocationOutput, NodeDescriptor};

/// 通过 Miresidian 运行时暴露的具体设备适配器。
///
/// 实现接收平台传来的兼容文本值。后续 `#[miresidian::operation]` 宏将为普通驱动
/// 生成强类型解析和描述元数据。
pub trait DeviceNode: Default + Send + 'static {
    /// 返回此节点类型的编译期元数据。
    fn descriptor() -> &'static NodeDescriptor;

    /// 使用初始化配置建立设备连接。
    fn init_device(&mut self, config: &[String]) -> DriverResult<InvocationOutput>;

    /// 释放节点持有的全部资源。
    fn close_device(&mut self) -> DriverResult<()>;

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
    fn get_port_no(&mut self) -> DriverResult<Option<String>> {
        Ok(None)
    }
}
