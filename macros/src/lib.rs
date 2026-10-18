//! Miresidian 属性过程宏的公开入口。
//!
//! 入口层只负责将 TokenStream 路由到对应实现；节点分析、代码生成和辅助规则分别位于
//! 独立模块，避免过程宏入口承载具体实现细节。

mod node;
mod node_impl;

use proc_macro::TokenStream;
use syn::{ItemImpl, parse_macro_input};

/// 标记一个可由 Miresidian 加载的节点结构体。
///
/// 参数必须是节点在测试平台中的展示名称，例如 `#[miresidian::node("串口通信")]`。
/// `node_impl` 会读取该名称并写入节点描述信息。
#[proc_macro_attribute]
pub fn node(attribute: TokenStream, item: TokenStream) -> TokenStream {
    node::expand(attribute, item)
}

/// 分析节点实现并生成平台调度与 C ABI 入口。
///
/// 标注的 `impl` 块必须包含 `init_device` 和 `close_device`，并且二者以 `&mut self`
/// 为接收者。宏会消费其中的 `operation`、`config`、`input` 与 `output` 标注。
#[proc_macro_attribute]
pub fn node_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_impl = parse_macro_input!(item as ItemImpl);
    match node_impl::expand_node_impl(&mut item_impl) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// 标记可由测试平台调用的节点方法；由 `node_impl` 消费。
///
/// 不带参数时，方法名会从 `snake_case` 自动转换为平台的 PascalCase；带一个字符串参数时，
/// 该字符串就是平台可见操作名称。
#[proc_macro_attribute]
pub fn operation(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记初始化配置参数；由 `node_impl` 消费。
///
/// 仅用于 `init_device` 的非 `self` 参数。参数可以是 `"名称"`，也可以是
/// `"名称:默认值"`；省略标注时使用参数名转换得到的平台名称。
#[proc_macro_attribute]
pub fn config(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记节点方法输入参数；由 `node_impl` 消费。
///
/// 用法与 `config` 相同，但应用于 `close_device`、`get_port_no` 和 `operation` 方法的
/// 非 `self` 参数。
#[proc_macro_attribute]
pub fn input(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记节点方法输出；由 `node_impl` 消费。
///
/// 此标注写在方法上，参数为一个或多个输出名称。未标注且返回值不是 `MiresidianResult<()>` 时，
/// 自动生成名为“返回值”的单个输出。
#[proc_macro_attribute]
pub fn output(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}
