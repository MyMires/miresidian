//! Miresidian 属性过程宏的公开入口。
//!
//! 入口层只负责将 TokenStream 路由到对应实现；节点分析、代码生成和辅助规则分别位于
//! 独立模块，避免过程宏入口承载具体实现细节。

mod framework;
mod node;
mod node_enum;
mod operation;

use proc_macro::TokenStream;

/// 为无字段业务枚举生成 NodeInput、NodeOutput、集合转换和结构化候选。
///
/// 成员名作为实际值，不读取整数 discriminant；候选格式和输入表示由调用平台适配。
/// 成员可用 `#[node_enum(display = "空闲")]` 或 `#[node_enum("空闲")]` 声明显示值，
/// 省略时显示值与成员名相同。直接 codec 仍只解析和输出实际值。
/// 重复标注、空白显示值、未知键、非字符串及枚举级标注在编译期报错；
/// 分隔符与显示值歧义由具体平台校验，不在通用派生中施加 TB 语法限制。
/// 不要求 Clone/Copy/Default；不支持带数据、空或泛型枚举。
#[proc_macro_derive(NodeEnum, attributes(node_enum))]
pub fn node_enum(item: TokenStream) -> TokenStream {
    node_enum::expand(item)
}

/// 标记一个可由 Miresidian 加载的节点结构体。
///
/// 参数必须以节点在测试平台中的展示名称开始，例如 `#[miresidian::node("串口通信")]`。
/// 可选 `platforms(AdapterType, ...)` 将实现 `Default` 的平台适配器编译进该 DLL。
/// 同模块的 `operation` 独立生成方法契约；本宏汇总注册项并生成节点接口与 C ABI。
///
/// 0.29.0 将节点级 instances 统一改为 instance，不保留旧入口；参数级 instance(key/ignore) 保持。
/// `instance` / `instance()` 默认单对象顺序多通道，`instance(multiple, config)`
/// 明确多对象关联，可追加 `parallel` 或 `selectable(sequential)`。不声明时仍禁用多通道。
/// `lease` / `lease(required)` 固定租约，`lease(disabled)` 固定直连，
/// `lease(selectable(required))` 发布选择器且默认租约。旧长写继续支持。
/// 所需 Cargo feature 必须开启；重复、缺关联、非法策略或同组混用长短写会编译报错。
#[proc_macro_attribute]
pub fn node(attribute: TokenStream, item: TokenStream) -> TokenStream {
    node::expand(attribute, item)
}

/// 声明平台方法并独立生成参数转换、描述信息和调用包装。
///
/// 不带参数时，方法名会从 `snake_case` 自动转换为平台的 PascalCase；带一个字符串参数时，
/// 该字符串就是平台可见操作名称。共享调用的执行方式由 `InitDevice` 的框架配置统一决定，
/// 不在操作标注中重复声明。
/// 可选 get_port_no 未实现时，平台返回 `["0", "当前未实现端口发现能力"]`；
/// 原 `Option<String>` 签名保持，None 返回未找到原因，NotSupported 的说明进入第二项输出。
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
/// 非 `self` 参数。普通业务参数可用 `batch`、`format(hex)` 或
/// `format(utf8, hex, selector = "数据格式")`，旧 `format(formats = [...])` 仍有效。
/// 单格式禁止 selector、多格式必须 selector 且首项默认；重复格式、未知格式或混用语法会报错。
/// 格式转换只用于普通业务 Input/Output，仍须显式开启对应 feature。
#[proc_macro_attribute]
pub fn input(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 已废弃：节点租约使用 `#[node("名称", lease)]` 或 `lease(...)` 子配置，参数角色由 config 消费。
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
/// 自动生成名为“返回值”的单个输出。可使用与 Input 相同的 `format(hex)` 或
/// 多格式分组；选择器追加在方法输入末尾。旧 formats 数组语法和属性先后顺序保持兼容。
#[proc_macro_attribute]
pub fn output(attribute: TokenStream, item: TokenStream) -> TokenStream {
    operation::expand_output(attribute, item)
}
