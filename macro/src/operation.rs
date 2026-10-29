//! 方法级平台契约生成；生命周期直接生成桥接，业务方法提交独立注册项。

mod definition;
mod descriptor;
mod dispatch;
mod naming;

use definition::{NodeMethod, ParameterSource};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, ImplItemFn, parse_macro_input};

pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let arguments: proc_macro2::TokenStream = attribute.into();
    let attribute: Attribute = if arguments.is_empty() {
        syn::parse_quote!(#[operation])
    } else {
        syn::parse_quote!(#[operation(#arguments)])
    };
    let mut method = parse_macro_input!(item as ImplItemFn);
    let result = (|| {
        let definition = definition::parse(&attribute, &mut method)?;
        let generated = match definition {
            NodeMethod::Init(init) => {
                let configs = init
                    .parameters
                    .iter()
                    .filter(|p| p.source == ParameterSource::Config)
                    .cloned()
                    .collect::<Vec<_>>();
                let inputs = init
                    .parameters
                    .iter()
                    .filter(|p| p.source == ParameterSource::Input)
                    .cloned()
                    .collect::<Vec<_>>();
                let configs = descriptor::parameters(&configs);
                let inputs = descriptor::parameters(&inputs);
                let outputs = descriptor::outputs(&init.outputs);
                let call = dispatch::initialize(&init);
                let validate = dispatch::validate_init(&init);
                let lease_parameters = dispatch::lease_parameters(&init);
                quote! {
                    const __MIRESIDIAN_INIT_CONFIG: &'static [::miresidian::ParameterDescriptor] = &[#(#configs),*];
                    const __MIRESIDIAN_INIT_INPUTS: &'static [::miresidian::ParameterDescriptor] = &[#(#inputs),*];
                    const __MIRESIDIAN_INIT_OUTPUTS: &'static [::miresidian::ParameterDescriptor] = &[#(#outputs),*];
                    fn __miresidian_initialize(config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<(Self, ::miresidian::InvocationOutput)> {
                        #call
                    }
                    fn __miresidian_validate_init(config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<()> {
                        #validate
                    }
                    fn __miresidian_lease_parameters(config: &[::std::string::String]) -> ::miresidian::MiresidianResult<::std::vec::Vec<(::std::string::String, ::std::string::String)>> {
                        #lease_parameters
                    }
                }
            }
            NodeMethod::Close(close) => {
                let call = dispatch::close(&close);
                quote! {
                    fn __miresidian_close(&mut self, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<()> { #call }
                }
            }
            NodeMethod::PortProbe(probe) => {
                let inputs = descriptor::parameters(&probe.parameters);
                let call = dispatch::port_probe(&probe);
                quote! {
                    fn __miresidian_probe(config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<::core::option::Option<::std::string::String>> { #call }
                    const __MIRESIDIAN_REGISTER_PROBE: () = {
                        ::miresidian::__private::inventory::submit! {
                            __MiresidianPortProbe { inputs: &[#(#inputs),*], call: __MiresidianNode::__miresidian_probe }
                        }
                    };
                }
            }
            NodeMethod::Operation(operation) => {
                let wrapper = format_ident!("__miresidian_dispatch_{}", operation.method);
                let registration = format_ident!("__MIRESIDIAN_REGISTER_{}", operation.method);
                let descriptor = descriptor::operation(&operation);
                let call = dispatch::operation(&operation);
                quote! {
                    fn #wrapper(node: &mut Self, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<::miresidian::InvocationOutput> { #call }
                    #[allow(non_upper_case_globals)]
                    const #registration: () = {
                        let _: fn(&mut Self, &[::std::string::String], &[::std::string::String]) -> ::miresidian::MiresidianResult<::miresidian::InvocationOutput> = __MiresidianNode::#wrapper;
                        ::miresidian::__private::inventory::submit! {
                            __MiresidianOperation {
                                descriptor: #descriptor,
                                call: __MiresidianNode::#wrapper,
                                source: (file!(), line!(), column!()),
                            }
                        }
                    };
                }
            }
        };
        // Copy conditional compilation onto every generated associated item, so a
        // disabled method cannot leave a dangling registration behind.
        let mut generated =
            syn::parse2::<syn::ItemImpl>(quote!(impl __MiresidianNode { #generated }))?;
        let cfg = crate::framework::conditional_attributes(&method.attrs);
        for item in &mut generated.items {
            match item {
                syn::ImplItem::Fn(item) => item.attrs.extend(cfg.clone()),
                syn::ImplItem::Const(item) => item.attrs.extend(cfg.clone()),
                _ => {}
            }
        }
        let items = generated.items;
        crate::framework::rewrite_paths(quote!(#method #(#items)*))
    })();
    result
        .unwrap_or_else(|error: syn::Error| error.into_compile_error())
        .into()
}

/// Preserve output annotations even when they precede operation in source order.
pub(crate) fn expand_output(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let Ok(mut method) = syn::parse::<ImplItemFn>(item.clone()) else {
        return item;
    };
    let Some(index) = method.attrs.iter().position(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "operation")
    }) else {
        return item;
    };
    let operation = method.attrs.remove(index);
    let arguments = match operation.meta {
        syn::Meta::Path(_) => proc_macro2::TokenStream::new(),
        syn::Meta::List(list) => list.tokens,
        meta => {
            return syn::Error::new_spanned(meta, "operation 参数格式无效")
                .into_compile_error()
                .into();
        }
    };
    let output: proc_macro2::TokenStream = attribute.into();
    method.attrs.push(syn::parse_quote!(#[output(#output)]));
    expand(arguments.into(), quote!(#method).into())
}
