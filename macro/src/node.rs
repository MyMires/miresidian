//! 结构体级节点声明；生成类型绑定、注册表、运行时适配与 C ABI。

mod arguments;
mod ffi;
mod generate;

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
        let cfg = crate::framework::conditional_attributes(&item.attrs);
        for item in &mut generated.items {
            match item {
                syn::Item::Type(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Struct(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Macro(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Impl(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Static(item) => item.attrs.extend(cfg.clone()),
                syn::Item::Fn(item) => item.attrs.extend(cfg.clone()),
                _ => {}
            }
        }
        crate::framework::rewrite_paths(quote::quote!(#generated))
    })();
    result
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}
