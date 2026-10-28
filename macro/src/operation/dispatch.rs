//! 参数转换与 `DeviceNode` 调度分支的令牌生成。

use quote::{format_ident, quote};

use super::definition::{FixedMethod, InitMethod, OperationMethod, Parameter, ParameterSource};

/// 构造节点后编码原有业务输出；编码失败时由框架清理已构造节点。
pub(super) fn initialize(method: &InitMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let identifier = &method.method;
    let construct = if method.has_output {
        quote!(let (node, value) = Self::#identifier(#(#variables),*)?;)
    } else {
        quote!(let node = Self::#identifier(#(#variables),*)?; let value = ();)
    };
    quote! {
        #(#arguments)*
        #construct
        ::miresidian::__private::finish_initialization(node, value, config)
    }
}

pub(super) fn close(method: &FixedMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let identifier = &method.method;
    quote! { #(#arguments)* Self::#identifier(self, #(#variables),*)?; Ok(()) }
}

/// 生成所有初始化 Config 的 Rust 名称与规范化值，供结构体级 lease 声明筛选。
pub(super) fn lease_parameters(method: &InitMethod) -> proc_macro2::TokenStream {
    let mut config_index = 0usize;
    let parameters = method.parameters.iter().filter_map(|parameter| {
        if parameter.source != ParameterSource::Config {
            return None;
        }
        let source_index = config_index;
        config_index += 1;
        let rust_name = &parameter.rust_name;
        let name = &parameter.name;
        let definition = &parameter.definition;
        let default = match &parameter.default {
            Some(value) => quote!(Some(#value)),
            None => quote!(None),
        };
        Some(quote! {
            values.push((
                #rust_name.to_owned(),
                ::miresidian::codec::decode_value_with_definition::<::std::string::String>(
                    config, #source_index, #name, #definition, #default,
                )?,
            ));
        })
    });
    quote! {
        let mut values = ::std::vec::Vec::new();
        #(#parameters)*
        Ok(values)
    }
}

/// 注册包装使用模块内的具体节点绑定，不依赖方法宏获知外层 impl 类型。
pub(super) fn operation(operation: &OperationMethod) -> proc_macro2::TokenStream {
    let method = &operation.method;
    let arguments = parse_arguments(&operation.parameters);
    let variables = argument_variables(operation.parameters.len());
    quote! {
        #(#arguments)*
        let value = Self::#method(node, #(#variables),*);
        ::miresidian::codec::encode_value(value?)
    }
}

pub(super) fn port_probe(method: &FixedMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let identifier = &method.method;
    quote! { #(#arguments)* Self::#identifier(#(#variables),*) }
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
            let definition = &parameter.definition;
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
            quote! { let #variable: #ty = ::miresidian::codec::decode_value_with_definition(#values, #source_index, #name, #definition, #default)?; }
        })
}

fn argument_variables(length: usize) -> impl Iterator<Item = syn::Ident> {
    (0..length).map(|index| format_ident!("__miresidian_argument_{index}"))
}
