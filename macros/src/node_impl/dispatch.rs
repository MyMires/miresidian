//! 参数转换与 `DeviceNode` 调度分支的令牌生成。

use super::definition::{FixedMethod, InitMethod, OperationMethod, Parameter};
use quote::{format_ident, quote};
use syn::Type;

/// 生成初始化配置解析和方法调用。
pub(super) fn with_config(self_type: &Type, method: &InitMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(
        &method.parameters,
        quote!(config),
        quote!(::miresidian::parse_config),
    );
    let variables = argument_variables(method.parameters.len());
    let method = &method.method;
    quote! { #(#arguments)* let value = <#self_type>::#method(self, #(#variables),*); ::miresidian::into_invocation_output(value?) }
}
/// 生成固定入口的输入解析和方法调用。
pub(super) fn with_input(self_type: &Type, method: &FixedMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(
        &method.parameters,
        quote!(input),
        quote!(::miresidian::parse_input),
    );
    let variables = argument_variables(method.parameters.len());
    let method = &method.method;
    quote! { #(#arguments)* <#self_type>::#method(self, #(#variables),*) }
}
/// 生成 `DeviceNode::invoke` 中的一个操作分支。
pub(super) fn operation(self_type: &Type, operation: &OperationMethod) -> proc_macro2::TokenStream {
    let operation_name = &operation.name;
    let method = &operation.method;
    let arguments = parse_arguments(
        &operation.parameters,
        quote!(input),
        quote!(::miresidian::parse_input),
    );
    let variables = argument_variables(operation.parameters.len());
    quote! { #operation_name => { #(#arguments)* let value = <#self_type>::#method(self, #(#variables),*); ::miresidian::into_invocation_output(value?) } }
}
/// 生成可选 `get_port_no` 的临时节点调用实现。
pub(super) fn port_probe(self_type: &Type, method: &FixedMethod) -> proc_macro2::TokenStream {
    let call = with_input(self_type, method);
    quote! { fn get_port_no(&mut self, input: &[::std::string::String]) -> ::miresidian::DriverResult<::core::option::Option<::std::string::String>> { #call } }
}
fn parse_arguments(
    parameters: &[Parameter],
    values: proc_macro2::TokenStream,
    parser: proc_macro2::TokenStream,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    parameters
        .iter()
        .enumerate()
        .map(move |(index, parameter)| {
            let variable = format_ident!("__miresidian_argument_{index}");
            let ty = &parameter.ty;
            let name = &parameter.name;
            let default = match &parameter.default {
                Some(value) => quote!(Some(#value)),
                None => quote!(None),
            };
            quote! { let #variable: #ty = #parser(#values, #index, #name, #default)?; }
        })
}
fn argument_variables(length: usize) -> impl Iterator<Item = syn::Ident> {
    (0..length).map(|index| format_ident!("__miresidian_argument_{index}"))
}
