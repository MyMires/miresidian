//! Miresidian 属性过程宏的公开入口。
//!
//! 入口层只负责将 TokenStream 路由到对应实现；节点分析、代码生成和辅助规则分别位于
//! 独立模块，避免过程宏入口承载具体实现细节。

mod framework;
mod node;
mod operation;

use proc_macro::TokenStream;

/// 标记一个可由 Miresidian 加载的节点结构体。
///
/// 参数必须以节点在测试平台中的展示名称开始，例如 `#[miresidian::node("串口通信")]`。
/// 可选 `platforms(AdapterType, ...)` 将实现 `Default` 的平台适配器编译进该 DLL。
/// 同模块的 `operation` 独立生成方法契约；本宏汇总注册项并生成节点接口与 C ABI。
#[proc_macro_attribute]
pub fn node(attribute: TokenStream, item: TokenStream) -> TokenStream {
    node::expand(attribute, item)
}

/// 声明平台方法并独立生成参数转换、描述信息和调用包装。
///
/// 不带参数时，方法名会从 `snake_case` 自动转换为平台的 PascalCase；带一个字符串参数时，
/// 该字符串就是平台可见操作名称。共享调用的执行方式由 `InitDevice` 的框架配置统一决定，
/// 不在操作标注中重复声明。
#[proc_macro_attribute]
pub fn operation(attribute: TokenStream, item: TokenStream) -> TokenStream {
    operation::expand(attribute, item)
}

/// 标记方法的 Config 参数；由 `operation` 消费。
///
/// 参数描述为 `"名称"` 或 `"名称:平台描述"`，不推导业务运行时默认值。
/// 初始化未标注参数默认来自 Config；其他方法可用本属性显式选择 Config 来源。
/// 仅初始化 Config 支持嵌套 `instance(key/ignore)`、`lease(key/ignore)`。
/// 两套角色分别控制实例组合身份与资源租约，节点整体策略仍在 `node` 上。
#[proc_macro_attribute]
pub fn config(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记节点方法输入参数；由 `operation` 消费。
///
/// 用法与 `config` 相同，但应用于 `close_device`、`get_port_no` 和 `operation` 方法的
/// 非 `self` 参数。
#[proc_macro_attribute]
pub fn input(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 已废弃：租约必须写为 `#[node("名称", lease(...))]` 子配置。
#[proc_macro_attribute]
pub fn lease(_attribute: TokenStream, _item: TokenStream) -> TokenStream {
    syn::Error::new(
        proc_macro2::Span::call_site(),
        "租约必须写为 #[miresidian::node(\"名称\", lease(...))]",
    )
    .into_compile_error()
    .into()
}

/// 标记节点方法输出；由 `operation` 消费。
///
/// 此标注写在方法上，参数为一个或多个输出名称。未标注且返回值不是 `MiresidianResult<()>` 时，
/// 自动生成名为“返回值”的单个输出。
#[proc_macro_attribute]
pub fn output(attribute: TokenStream, item: TokenStream) -> TokenStream {
    operation::expand_output(attribute, item)
}
