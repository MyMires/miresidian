//! 参数转换与 `DeviceNode` 调度分支的令牌生成。

use quote::{format_ident, quote};

use super::definition::{FixedMethod, InitMethod, OperationMethod, Parameter, ParameterSource};

/// 构造节点后编码原有业务输出；编码失败时由框架清理已构造节点。
pub(super) fn initialize(method: &InitMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let identifier = &method.method;
    let invocation = if method.asynchronous {
        quote!(::miresidian::__private::block_on(Self::#identifier(#(#variables),*)))
    } else {
        quote!(Self::#identifier(#(#variables),*))
    };
    let construct = if method.has_output {
        quote!(let (node, value) = #invocation?;)
    } else {
        quote!(let node = #invocation?; let value = ();)
    };
    quote! {
        #(#arguments)*
        #construct
        ::miresidian::__private::finish_initialization(node, value, config)
    }
}

/// 单例替换前只解析参数，不创建资源；已解析值在预校验结束后释放。
pub(super) fn validate_init(method: &InitMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    quote! { #(#arguments)* let _ = (#(#variables,)*); Ok(()) }
}

pub(super) fn close(method: &FixedMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let identifier = &method.method;
    let call = if method.asynchronous {
        quote!(::miresidian::__private::block_on(Self::#identifier(self,#(#variables),*)))
    } else {
        quote!(Self::#identifier(self,#(#variables),*))
    };
    quote! { #(#arguments)* #call?; Ok(()) }
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
    if operation.parameters.iter().any(|parameter| parameter.batch) {
        return batch_operation(operation, false);
    }
    let method = &operation.method;
    let arguments = parse_arguments(&operation.parameters);
    let variables = argument_variables(operation.parameters.len());
    let call = if operation.asynchronous {
        quote!(::miresidian::__private::block_on(Self::#method(node,#(#variables),*)))
    } else {
        quote!(Self::#method(node,#(#variables),*))
    };
    quote! { #(#arguments)* let value=#call; ::miresidian::codec::encode_value(value?) }
}

pub(super) fn operation_async(operation: &OperationMethod) -> proc_macro2::TokenStream {
    if operation.parameters.iter().any(|parameter| parameter.batch) {
        return batch_operation(operation, true);
    }
    let method = &operation.method;
    let arguments = parse_arguments(&operation.parameters);
    let variables = argument_variables(operation.parameters.len());
    quote! { Box::pin(async move { #(#arguments)* let value=Self::#method(node,#(#variables),*).await; ::miresidian::codec::encode_value(value?) }) }
}

/// 批量操作仍向业务方法传递标量。宏在入口解析整数集合、按位置广播后依次重复原调用。
fn batch_operation(operation: &OperationMethod, asynchronous: bool) -> proc_macro2::TokenStream {
    let method = &operation.method;
    let batch_values = parse_batch_values(&operation.parameters);
    let batch_lengths = operation
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, parameter)| parameter.batch)
        .map(|(index, parameter)| {
            let variable = format_ident!("__miresidian_batch_values_{index}");
            let name = &parameter.name;
            quote!((#name, #variable.len()))
        });
    let arguments = parse_batch_arguments(&operation.parameters);
    let variables = argument_variables(operation.parameters.len());
    let call = if asynchronous {
        quote!(Self::#method(node,#(#variables),*).await)
    } else {
        quote!(Self::#method(node,#(#variables),*))
    };
    let body = quote! {
        ::miresidian::__require_batch_feature();
        #(#batch_values)*
        let __miresidian_batch_length = ::miresidian::__private::batch_length(&[#(#batch_lengths),*])?;
        let mut __miresidian_batch_outputs = ::std::vec::Vec::with_capacity(__miresidian_batch_length);
        for __miresidian_batch_index in 0..__miresidian_batch_length {
            #(#arguments)*
            let __miresidian_batch_value = #call?;
            __miresidian_batch_outputs.push(::miresidian::codec::encode_value(__miresidian_batch_value)?);
        }
        ::miresidian::__private::batch_outputs(__miresidian_batch_outputs)
    };
    if asynchronous {
        quote!(Box::pin(async move { #body }))
    } else {
        body
    }
}

/// 解析每个批量 Input 为整数数组；普通参数在每轮调用内按原规则解析。
fn parse_batch_values(
    parameters: &[Parameter],
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    let mut input_index = 0usize;
    parameters.iter().enumerate().filter_map(move |(index, parameter)| {
        let source_index = match parameter.source {
            ParameterSource::Config => return None,
            ParameterSource::Input => {
                let current = input_index;
                input_index += 1;
                current
            }
        };
        if !parameter.batch {
            return None;
        }
        let variable = format_ident!("__miresidian_batch_values_{index}");
        let ty = &parameter.ty;
        let name = &parameter.name;
        let definition = &parameter.definition;
        let default = match &parameter.default {
            Some(value) => quote!(Some(#value)),
            None => quote!(None),
        };
        Some(quote! {
            let #variable: ::std::vec::Vec<#ty> = ::miresidian::codec::decode_value_with_definition(
                input, #source_index, #name, #definition, #default,
            )?;
        })
    })
}

fn parse_batch_arguments(
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
            if parameter.batch {
                // batch 只能来自 Input；仍须推进该通道的索引，保证后续普通 Input
                // 与平台参数位置保持一致。
                input_index += 1;
                let batch_values = format_ident!("__miresidian_batch_values_{index}");
                return quote! {
                    let #variable: #ty = #batch_values[
                        if #batch_values.len() == 1 { 0 } else { __miresidian_batch_index }
                    ].clone();
                };
            }
            let name = &parameter.name;
            let definition = &parameter.definition;
            let default = match &parameter.default {
                Some(value) => quote!(Some(#value)),
                None => quote!(None),
            };
            let (values, source_index) = match parameter.source {
                ParameterSource::Config => {
                    let current = config_index;
                    config_index += 1;
                    (quote!(config), current)
                }
                ParameterSource::Input => {
                    let current = input_index;
                    input_index += 1;
                    (quote!(input), current)
                }
            };
            quote! {
                let #variable: #ty = ::miresidian::codec::decode_value_with_definition(
                    #values, #source_index, #name, #definition, #default,
                )?;
            }
        })
}

pub(super) fn port_probe(method: &FixedMethod) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(&method.parameters);
    let variables = argument_variables(method.parameters.len());
    let identifier = &method.method;
    let call = if method.asynchronous {
        quote!(::miresidian::__private::block_on(Self::#identifier(#(#variables),*)))
    } else {
        quote!(Self::#identifier(#(#variables),*))
    };
    quote! { #(#arguments)* #call }
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
