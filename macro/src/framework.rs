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
            // inventory initializers are expressions inside opaque macro tokens;
            // ordinary path visitors do not reach the generated descriptor paths.
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

/// Retain only conditional compilation, not unrelated cfg_attr payloads such as derive.
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
