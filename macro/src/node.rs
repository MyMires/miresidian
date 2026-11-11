//! 结构体级节点声明；生成类型绑定、注册表、运行时适配与 C ABI。
//!
//! 方法签名由各 operation 宏独立解析，本模块不扫描源文件或 impl。
//! 固定模块别名将方法注册绑定到节点；生成项随节点条件编译一起启用或移除。

mod arguments;
mod ffi;
mod generate;
mod instances;
mod lease;

use proc_macro::TokenStream;
use syn::{Error, ItemStruct, parse_macro_input};

pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = parse_macro_input!(attribute as arguments::NodeArguments);
    let item = parse_macro_input!(item as ItemStruct);
    let result = (|| {
        if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
            return Err(Error::new_spanned(
                &item.generics,
                "节点结构体不能带泛型参数",
            ));
        }
        if item.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|s| s.ident == "lease")
        }) {
            return Err(Error::new_spanned(
                &item,
                "lease 必须写为 #[node(\"名称\", lease(...))] 的子配置",
            ));
        }
        let mut generated = syn::parse2::<syn::File>(generate::expand(&item, &arguments))?;
        // 仅复制条件编译项，不能把节点的 derive 等属性传播到导出函数、别名或注册表。
        let cfg = crate::framework::conditional_attributes(&item.attrs);
        for item in &mut generated.items {
            match item {
                syn::Item::Type(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Struct(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Macro(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Impl(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Static(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Const(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Fn(item) => item.attrs.extend(cfg.clone()),
                _ => {}
            }
        }
        // 所有生成完成后统一处理依赖重命名，避免注册项和 ABI 桥使用不同框架路径。
        crate::framework::rewrite_paths(quote::quote!(#generated))
    })();
    result
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}
