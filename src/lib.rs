//! Miresidian 是供宿主加载通用业务节点的运行时框架。
//!
//! 框架提供稳定调用契约、生命周期、属性宏与 C ABI，并在通道边界隔离可展开 panic。
//!
//! C ABI v10 仅生成 `miresidian_invoke(const char *request_json)` 调用及独立 ABI 版本查询。
//! 请求含必填非空 `function`、可省略的字符串数组 `config`/`input`（默认 `[]`），以及
//! 可选 u32 数值 `platform`；省略或为 0 固定使用 DLL 默认平台，非零显式选择已注册平台。
//! Config 不包含平台选择项。调用方应在同线程下一次调用前复制 DLL 返回文本，不得释放。
//!
//! `#[derive(miresidian::NodeEnum)]` 为无字段业务枚举生成 codec 与结构化候选；
//! 参数没有非空描述时，由本次平台适配器渲染并配套规范化输入，不提供缺参首项默认值。
//!
//! 0.28.0 起支持 feature 短写：`instances` 默认单对象顺序多通道，
//! `instances(multiple, config)` 显式多对象关联；裸 `lease` 等价于固定 required。
//! `format(hex)` 与 `format(utf8, hex, selector = "格式")` 可用于普通业务 Input/Output，
//! 仍与 `batch` 独立组合。旧完整标注、feature 门禁和平台槽位顺序保持兼容。
//!
//! # 节点声明示例
//!
//! 下例是一个最小可发布节点。驱动作者只声明状态与方法标注；函数列表、类型转换、生命周期
//! 调度和 DLL 的 C ABI 导出均由宏生成。
//!
//! ```no_run
//! use miresidian::MiresidianResult;
//!
//! #[miresidian::node("示例节点")]
//! struct ExampleNode;
//!
//! impl ExampleNode {
//!     #[miresidian::operation]
//!     #[miresidian::output("状态")]
//!     fn init_device() -> MiresidianResult<(Self, String)> {
//!         Ok((Self, "已连接".to_owned()))
//!     }
//!
//!     #[miresidian::operation]
//!     fn close_device(&mut self) -> MiresidianResult<()> {
//!         Ok(())
//!     }
//!
//!     #[miresidian::operation("回显")]
//!     #[miresidian::output("接收文本")]
//!     fn echo(&mut self, #[miresidian::input("发送文本")] text: String) -> MiresidianResult<String> {
//!         Ok(text)
//!     }
//! }
//! ```

#[doc(hidden)]
#[path = "macro_support.rs"]
pub mod __private;

pub mod interop;
pub mod platform;
mod runtime;

/// 租约通用模型及可选的本地 IPC、共享资源能力。
///
/// 通用模型始终可用；Cargo feature `lease` 才启用本地 IPC 与共享资源后端。
/// 它不定义具体设备协议，也不替代节点对访问控制、身份验证和资源一致性的责任。
pub use miresidian_lease as lease;

/// batch feature 的参数对齐与输出聚合实现。节点一般只通过参数上的 `batch` 标记接入。
#[cfg(feature = "batch")]
#[doc(hidden)]
pub use miresidian_batch as batch;

/// 参数级顺序批量执行。只有启用 Cargo feature `batch` 并在普通业务 Input 参数上写入
/// `#[miresidian::input("名称", batch)]` 时，宏按原参数类型展开记录；
/// 字符串和布尔无需 format，整数支持列表、范围及混合形式。
#[cfg(feature = "batch")]
#[doc(hidden)]
pub const fn __require_batch_feature() {}

/// 仅当 Cargo feature `lease` 启用时由租约节点引用；缺失该符号会使误用的租约声明在编译期失败。
#[cfg(feature = "lease")]
#[doc(hidden)]
pub fn __lease_feature_required() {}

/// 宏通过依赖内的符号检查 feature，避免误检查节点 crate 自身的同名 feature。
#[cfg(feature = "instances")]
#[doc(hidden)]
pub const fn __require_instances_runtime() {}

/// 显式格式编解码增强；启用 Cargo feature `format` 后可用。
#[cfg(feature = "format")]
pub mod format {
    pub use miresidian_format::{
        FormatInput, FormatOutput, decode_formatted, decode_formatted_batch, encode_formatted,
        select_format, select_format_value,
    };
}

/// 当前编译进节点 DLL 的 Miresidian 框架版本。
pub const MIRESIDIAN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 框架级数据编解码 API。
///
/// 属性宏生成的节点调度代码直接调用此模块，因此全部节点共用同一套类型、数组、字节和字符
/// 编码规则。根 crate 同时保留同名重导出，以兼容已有 Rust 调用代码。
pub mod codec {
    pub use miresidian_codec::{
        ByteTextFormat, EnumOption, NodeCollectionElement, NodeInput, NodeOutput,
        decode_text_to_bytes, decode_value, decode_value_with_definition, encode_bytes_to_text,
        encode_value, normalize_enum_text, normalize_value_with_definition, parse_byte_text_format,
        transcode_text,
    };
}

pub use interop::{ffi, ffi_runtime};
pub use miresidian_abi::{
    MIRESIDIAN_ABI_VERSION, MIRESIDIAN_PLATFORM_DEFAULT, MIRESIDIAN_STATUS_INTERNAL_ERROR,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED,
    MiresidianStatus, MiresidianTextArray,
};
pub use miresidian_codec::{
    ByteTextFormat, EnumOption, ErrorCategory, InvocationOutput, MiresidianError, MiresidianResult,
    NodeCollectionElement, NodeInput, NodeOutput, NodeScalar, NodeValue, decode_text_to_bytes,
    decode_value, decode_value_with_definition, encode_bytes_to_text, encode_value,
    normalize_enum_text, normalize_value_with_definition, parse_byte_text_format, transcode_text,
};
/// 供具体节点使用的属性宏。
///
/// 这是框架节点项目的必选代码生成能力。
pub use miresidian_macro::{NodeEnum, config, input, lease, node, operation, output};
pub use platform::{
    MultiChannelInvocation, MultiChannelResponse, Platform, PlatformAdapter, PlatformRegistry,
    RuntimeInvocation, RuntimeResponse, SharedInvocation, SharedResponse, TB_LV_TEST_PLATFORM,
    TbLvTestPlatformAdapter,
};
pub use runtime::{
    CHANNEL_REUSED_MESSAGE, CLOSE_ALL_DEVICES_OPERATION, CLOSE_DEVICE_OPERATION, CallContext,
    ExecutionLimits, ExecutionMode, ExecutionPolicy, GET_FRAMEWORK_VERSION_OPERATION,
    GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION,
    GET_PROJECT_VERSION_OPERATION, INIT_DEVICE_OPERATION, InstanceBinding, InstanceMode,
    InstancePolicy, Invocation, LeaseAccess, LeaseActivation, Node, NodeDescriptor, NodeFuture,
    NodeRuntime, OperationDescriptor, PLATFORM_ADAPTER_NAME, ParameterDescriptor,
    SHARED_INVOCATION_MARKER, TaskService, TaskSnapshot, TaskState, shutdown_executor,
};

/// 宏通过框架符号检查显式格式转换能力。
#[cfg(feature = "format")]
#[doc(hidden)]
pub const fn __require_format_feature() {}
