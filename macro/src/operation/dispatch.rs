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

/// 同步桥接与原生 async 桥接共用一套参数布局和输出转换。
pub(super) fn operation(operation: &OperationMethod) -> proc_macro2::TokenStream {
    operation_body(operation, false)
}
pub(super) fn operation_async(operation: &OperationMethod) -> proc_macro2::TokenStream {
    let body = operation_body(operation, true);
    quote!(Box::pin(async move { #body }))
}

fn operation_body(operation: &OperationMethod, asynchronous: bool) -> proc_macro2::TokenStream {
    let has_batch = operation.parameters.iter().any(|p| p.batch);
    let has_codec = operation.parameters.iter().any(|p| p.codec.is_some())
        || operation.output_codecs.iter().any(Option::is_some);
    let mut prepare = Vec::new();
    let mut arguments = Vec::new();
    let mut lengths = Vec::new();
    let mut config_index = 0usize;
    let mut input_index = 0usize;
    for (index, parameter) in operation.parameters.iter().enumerate() {
        let variable = format_ident!("__miresidian_argument_{index}");
        let stored = format_ident!("__miresidian_values_{index}");
        let ty = &parameter.ty;
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
        let parse = if let Some(codec) = &parameter.codec {
            let selection = codec.selection(input_index);
            input_index += usize::from(codec.selector.is_some());
            let format = format_ident!("__miresidian_input_format_{index}");
            prepare.push(quote!(let #format = #selection;));
            let decode = if parameter.batch {
                quote!(::miresidian::codec::decode_formatted_batch::<#ty>)
            } else {
                quote!(::miresidian::codec::decode_formatted::<#ty>)
            };
            quote!({
                let text = ::miresidian::codec::normalize_value_with_definition(#values, #source_index, #name, #definition, #default)?;
                #decode(&text, #format, #name)?
            })
        } else {
            let parsed_type = if parameter.batch {
                quote!(::std::vec::Vec<#ty>)
            } else {
                quote!(#ty)
            };
            quote!(::miresidian::codec::decode_value_with_definition::<#parsed_type>(#values, #source_index, #name, #definition, #default)?)
        };
        if parameter.batch || !has_batch {
            prepare.push(quote!(let #stored = #parse;));
        }
        if parameter.batch {
            lengths.push(quote!((#name, #stored.len())));
            arguments.push(quote!(let #variable: #ty = #stored[if #stored.len() == 1 { 0 } else { __miresidian_batch_index }].clone();));
        } else if has_batch {
            arguments.push(quote!(let #variable: #ty = #parse;));
        } else {
            arguments.push(quote!(let #variable: #ty = #stored;));
        }
    }
    let mut encoded = Vec::new();
    let mut output_variables = Vec::new();
    for (index, (ty, codec)) in operation
        .output_types
        .iter()
        .zip(&operation.output_codecs)
        .enumerate()
    {
        let variable = format_ident!("__miresidian_output_{index}");
        output_variables.push(variable.clone());
        if let Some(codec) = codec {
            let selection = codec.selection(input_index);
            input_index += usize::from(codec.selector.is_some());
            let format = format_ident!("__miresidian_output_format_{index}");
            prepare.push(quote!(let #format = #selection;));
            encoded.push(quote!(__miresidian_encoded.push(::miresidian::codec::encode_formatted::<#ty>(#variable, #format)?);));
        } else {
            encoded.push(quote!(__miresidian_encoded.extend(::miresidian::codec::encode_value(#variable)?.into_values());));
        }
    }
    let encode = if operation.output_codecs.iter().any(Option::is_some) {
        let destructure = if !operation.output_tuple {
            let variable = &output_variables[0];
            quote!(let #variable = __miresidian_value;)
        } else {
            quote!(let (#(#output_variables,)*) = __miresidian_value;)
        };
        quote!({ #destructure let mut __miresidian_encoded = ::std::vec::Vec::new(); #(#encoded)* ::miresidian::InvocationOutput::new(__miresidian_encoded) })
    } else {
        quote!(::miresidian::codec::encode_value(__miresidian_value)?)
    };
    let method = &operation.method;
    let variables = argument_variables(operation.parameters.len());
    let call = if asynchronous {
        quote!(Self::#method(node, #(#variables),*).await)
    } else if operation.asynchronous {
        quote!(::miresidian::__private::block_on(Self::#method(node, #(#variables),*)))
    } else {
        quote!(Self::#method(node, #(#variables),*))
    };
    let feature = has_codec.then(|| quote!(::miresidian::__require_codec_feature();));
    if has_batch {
        quote! {
            #feature ::miresidian::__require_batch_feature(); #(#prepare)*
            let __miresidian_batch_length = ::miresidian::__private::batch_length(&[#(#lengths),*])?;
            let mut __miresidian_batch_outputs = ::std::vec::Vec::with_capacity(__miresidian_batch_length);
            for __miresidian_batch_index in 0..__miresidian_batch_length {
                #(#arguments)* let __miresidian_value = #call?;
                __miresidian_batch_outputs.push(#encode);
            }
            ::miresidian::__private::batch_outputs(__miresidian_batch_outputs)
        }
    } else {
        let result = if operation.output_codecs.iter().any(Option::is_some) {
            quote!(Ok(#encode))
        } else {
            quote!(::miresidian::codec::encode_value(__miresidian_value))
        };
        quote! { #feature #(#prepare)* #(#arguments)* let __miresidian_value = #call?; #result }
    }
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
