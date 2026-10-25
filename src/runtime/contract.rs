//! 运行时与后续 FFI 边界共用的文本调用类型。

/// 返回可调用操作签名的兼容操作。
pub const GET_FUNCTIONS_OPERATION: &str = "GetFunctions";
/// 返回当前 DLL 所使用框架版本的常驻查询。
pub const GET_FRAMEWORK_VERSION_OPERATION: &str = "GetFrameworkVersion";
/// 返回当前 DLL 项目版本的常驻查询。
pub const GET_PROJECT_VERSION_OPERATION: &str = "GetProjectVersion";
/// 返回初始化配置元数据的兼容操作。
pub const GET_INIT_CONFIG_OPERATION: &str = "GetInitConfig";
/// 探测节点端口、但不保留节点实例的兼容操作。
pub const GET_PORT_NO_OPERATION: &str = "GetPortNO";
/// 创建并初始化当前节点的兼容操作。
pub const INIT_DEVICE_OPERATION: &str = "InitDevice";
/// 关闭并释放当前节点的兼容操作。
pub const CLOSE_DEVICE_OPERATION: &str = "CloseDevice";
/// 关闭并释放当前运行时全部通道的兼容操作。
pub const CLOSE_ALL_DEVICES_OPERATION: &str = "CloseAllDevices";
/// TB LV Test Platform 兼容适配器使用的共享调用标记值。
///
/// 新平台不应依赖该文本标记，而应通过 `PlatformAdapter` 构造结构化共享调用。
pub const SHARED_INVOCATION_MARKER: &str = "共享";
/// InitDevice 命中已有通道而复用对象时返回的固定提示文本。
pub const CHANNEL_REUSED_MESSAGE: &str = "已复用已有连接";

/// 对 Miresidian 运行时发起的一次调用请求。
///
/// 本层刻意将 `config` 和 `input` 保留为文本，以便直接映射既有 C# 的
/// `CallFunction` 调用约定。后续由生成的包装代码负责验证并转换为 Rust 类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// 请求的操作名称，例如 `InitDevice` 或 `Query`。
    pub operation: String,
    /// 与本次调用关联的配置值。
    pub config: Vec<String>,
    /// 与本次调用关联的输入值。
    pub input: Vec<String>,
}

impl Invocation {
    /// 由平台侧的文本值列表创建调用请求。
    pub fn new(
        operation: impl Into<String>,
        config: impl Into<Vec<String>>,
        input: impl Into<Vec<String>>,
    ) -> Self {
        Self {
            operation: operation.into(),
            config: config.into(),
            input: input.into(),
        }
    }
}
