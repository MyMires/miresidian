//! 具体设备节点实现的扩展点。

use crate::{InvocationOutput, MiresidianResult, NodeDescriptor};

/// 由 `#[miresidian::node_impl]` 为具体驱动生成的运行时适配器。
///
/// 驱动作者不应手写此 trait。宏会从 `operation`、`config`、`input` 与 `output`
/// 标注生成实现、强类型转换和描述元数据。
pub trait DeviceNode: Default + Send + 'static {
    /// 返回此节点类型的编译期元数据。
    fn descriptor() -> &'static NodeDescriptor;

    /// 返回构建当前节点 DLL 的项目版本。
    ///
    /// 宏生成的节点实现会返回其 crate 的 `CARGO_PKG_VERSION`。手写实现若未覆盖
    /// 本方法，则显式表明其版本未声明。
    fn project_version() -> &'static str {
        "未声明"
    }

    /// 使用初始化调用建立设备连接。
    ///
    /// `config` 与 `input` 保留外部调用的两个独立参数通道。通常初始化参数来自
    /// `config`；驱动方法将参数标注为 `#[miresidian::input]` 时，该参数改从
    /// `input` 读取。两个切片的下标分别按各自通道中的参数顺序计算。
    fn init_device(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput>;

    /// 释放节点持有的全部资源。
    ///
    /// 默认情况下关闭参数来自 `input`。节点作者可通过参数上的 `config` 或
    /// `input` 标注指定实际通道，因此运行时必须原样传入两组参数。
    fn close_device(&mut self, config: &[String], input: &[String]) -> MiresidianResult<()>;

    /// 在成功初始化后运行节点自定义操作。
    ///
    /// 未标注的操作参数默认来自 `input`。参数上的 `#[miresidian::config]` 可使
    /// 该单个参数从 `config` 读取，二者可在同一操作中混用。
    fn invoke(
        &mut self,
        operation: &str,
        config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput>;

    /// 使用临时节点实例探测端口。
    ///
    /// 未标注参数默认来自 `input`，也可以通过方法参数标注改从 `config` 读取。
    ///
    /// 返回 `Ok(None)` 表示旧版 `GetPortNO` 结果 `["0", ""]`。
    fn get_port_no(
        &mut self,
        _config: &[String],
        _input: &[String],
    ) -> MiresidianResult<Option<String>> {
        Ok(None)
    }
}
