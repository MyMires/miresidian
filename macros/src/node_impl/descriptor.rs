//! 平台元数据描述的令牌生成。

use super::definition::{OperationMethod, Parameter, ParameterSource};
use quote::quote;

/// 生成 Config 或 Input 的平台描述项。
pub(super) fn parameters(
    parameters: &[Parameter],
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    parameters.iter().map(|parameter| {
        let name = &parameter.name;
        let default = match &parameter.default {
            Some(value) => quote!(Some(#value)),
            None => quote!(None),
        };
        quote!(::miresidian::ParameterDescriptor { name: #name, default_value: #default })
    })
}
/// 生成 Output 的平台描述项；输出没有默认值。
pub(super) fn outputs(
    outputs: &[syn::LitStr],
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    outputs
        .iter()
        .map(|name| quote!(::miresidian::ParameterDescriptor { name: #name, default_value: None }))
}
/// 生成一个完整的平台操作描述。
pub(super) fn operation(operation: &OperationMethod) -> proc_macro2::TokenStream {
    let name = &operation.name;
    let inputs = parameters(&operation.parameters);
    let outputs = outputs(&operation.outputs);
    let execution = execution_mode(operation.execution);
    let channel_parameters = operation
        .parameters
        .iter()
        .filter(|parameter| parameter.source == ParameterSource::Input)
        .cloned()
        .collect::<Vec<_>>();
    let channel_inputs = parameters(&channel_parameters);
    quote! {
        ::miresidian::OperationDescriptor {
            name: #name, inputs: &[#(#inputs),*], outputs: &[#(#outputs),*],
            execution: #execution, channel_inputs: &[#(#channel_inputs),*],
        }
    }
}
/// 生成共享执行方式的描述常量。
fn execution_mode(execution: super::definition::ExecutionMode) -> proc_macro2::TokenStream {
    match execution {
        super::definition::ExecutionMode::Sequential => {
            quote!(::miresidian::ExecutionMode::Sequential)
        }
        super::definition::ExecutionMode::Parallel => quote!(::miresidian::ExecutionMode::Parallel),
    }
}
