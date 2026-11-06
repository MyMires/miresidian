//! 宏生成代码的框架依赖路径解析，支持节点重命名 miresidian 依赖。

use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use syn::{Error, Result, visit_mut::VisitMut};

pub(crate) fn rewrite_paths(tokens: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
    let FoundCrate::Name(name) = crate_name("miresidian").map_err(|error| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!("无法定位 miresidian 依赖：{error}"),
        )
    })?
    else {
        return Ok(tokens);
    };
    let mut file = syn::parse2::<syn::File>(tokens)?;
    struct RenameFramework {
        name: syn::Ident,
    }
    impl VisitMut for RenameFramework {
        fn visit_macro_mut(&mut self, value: &mut syn::Macro) {
            // inventory 初始化表达式包在宏 token 内，普通语法路径遍历不会进入。
            // 须显式解析 submit 内容，才能同步重写注册描述中被重命名的框架依赖路径。
            if value
                .path
                .segments
                .iter()
                .any(|segment| segment.ident == "inventory")
                && value
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "submit")
                && let Ok(mut expression) = syn::parse2::<syn::Expr>(value.tokens.clone())
            {
                self.visit_expr_mut(&mut expression);
                value.tokens = quote!(#expression);
            }
            syn::visit_mut::visit_macro_mut(self, value);
        }
        fn visit_path_mut(&mut self, path: &mut syn::Path) {
            if path.leading_colon.is_some()
                && path
                    .segments
                    .first()
                    .is_some_and(|segment| segment.ident == "miresidian")
            {
                path.segments.first_mut().expect("checked above").ident = self.name.clone();
            }
            syn::visit_mut::visit_path_mut(self, path);
        }
    }
    RenameFramework {
        name: syn::Ident::new(&name, proc_macro2::Span::call_site()),
    }
    .visit_file_mut(&mut file);
    Ok(quote!(#file))
}

/// 提取 cfg 及 cfg_attr 内嵌的条件编译项，保留嵌套条件而丢弃 derive 等无关载荷。
/// 这些属性将复制到生成项，避免被禁用的节点或方法留下注册项与悬空引用。
pub(crate) fn conditional_attributes(attributes: &[syn::Attribute]) -> Vec<syn::Attribute> {
    fn conditional(meta: &syn::Meta) -> Option<syn::Meta> {
        if meta.path().is_ident("cfg") {
            return Some(meta.clone());
        }
        if !meta.path().is_ident("cfg_attr") {
            return None;
        }
        let syn::Meta::List(list) = meta else {
            return None;
        };
        let values = list
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
            .ok()?;
        let predicate = values.first()?;
        let nested = values
            .iter()
            .skip(1)
            .filter_map(conditional)
            .collect::<Vec<_>>();
        if nested.is_empty() {
            None
        } else {
            Some(syn::parse_quote!(cfg_attr(#predicate, #(#nested),*)))
        }
    }
    attributes
        .iter()
        .filter_map(|attribute| {
            let meta = conditional(&attribute.meta)?;
            Some(syn::parse_quote!(#[#meta]))
        })
        .collect()
}
