//! 参数转换与 `DeviceNode` 调度分支的令牌生成。

use quote::{format_ident, quote};
use syn::Type;

use super::definition::{FixedMethod, InitMethod, OperationMethod, Parameter, ParameterSource};

/// 生成初始化方法的双通道参数解析和调用。
pub(super) fn with_config(self_type: &Type, method: &InitMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let method = &method.method;
    quote! { #(#arguments)* let value = <#self_type>::#method(self, #(#variables),*); ::miresidian::codec::encode_value(value?) }
}

/// 生成固定入口的双通道参数解析和调用。
pub(super) fn with_input(self_type: &Type, method: &FixedMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let method = &method.method;
    quote! { #(#arguments)* <#self_type>::#method(self, #(#variables),*) }
}

/// 生成 `DeviceNode::invoke` 中的一个双通道操作分支。
pub(super) fn operation(self_type: &Type, operation: &OperationMethod) -> proc_macro2::TokenStream {
    let operation_name = &operation.name;
    let method = &operation.method;
    let arguments = parse_arguments(&operation.parameters);
    let variables = argument_variables(operation.parameters.len());
    quote! { #operation_name => { #(#arguments)* let value = <#self_type>::#method(self, #(#variables),*); ::miresidian::codec::encode_value(value?) } }
}

/// 生成可选 `get_port_no` 的双通道临时节点调用实现。
pub(super) fn port_probe(self_type: &Type, method: &FixedMethod) -> proc_macro2::TokenStream {
    let call = with_input(self_type, method);
    quote! { fn get_port_no(&mut self, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::DriverResult<::core::option::Option<::std::string::String>> { #call } }
}

/// 保持 Rust 方法参数顺序，同时分别维护 `config` 与 `input` 的位置索引。
fn parse_arguments(
    parameters: &[Parameter],
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    let mut config_index = 0usize;
    let mut input_index = 0usize;
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
            let (values, source_index) = match parameter.source {
                ParameterSource::Config => {
                    let source_index = config_index;
                    config_index += 1;
                    (
                        quote!(config),
                        source_index,
                    )
                }
                ParameterSource::Input => {
                    let source_index = input_index;
                    input_index += 1;
                    (
                        quote!(input),
                        source_index,
                    )
                }
            };
            quote! { let #variable: #ty = ::miresidian::codec::decode_value(#values, #source_index, #name, #default)?; }
        })
}

fn argument_variables(length: usize) -> impl Iterator<Item = syn::Ident> {
    (0..length).map(|index| format_ident!("__miresidian_argument_{index}"))
}
