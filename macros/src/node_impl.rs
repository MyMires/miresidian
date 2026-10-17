//! `#[node_impl]` 的编排层。
//!
//! 只负责串联解析、描述信息、调度和 C ABI 生成；具体规则按职责位于同名目录下。

mod definition;
mod descriptor;
mod dispatch;
mod ffi;
mod naming;

use quote::{format_ident, quote};
use syn::{Error, ItemImpl, Result, Type};

/// 分析节点实现块并生成平台调度与 C ABI 入口。
pub(crate) fn expand_node_impl(item_impl: &mut ItemImpl) -> Result<proc_macro2::TokenStream> {
    let self_type = (*item_impl.self_ty).clone();
    let self_identifier = type_identifier(&self_type)?;
    let prefix = format_ident!(
        "__MIRESIDIAN_{}",
        self_identifier.to_string().to_uppercase()
    );
    let init_config = format_ident!("{prefix}_INIT_CONFIG");
    let init_outputs = format_ident!("{prefix}_INIT_OUTPUTS");
    let port_probe_inputs = format_ident!("{prefix}_PORT_PROBE_INPUTS");
    let operations = format_ident!("{prefix}_OPERATIONS");
    let descriptor = format_ident!("{prefix}_DESCRIPTOR");
    let runtime = format_ident!("{prefix}_RUNTIME");

    let definition = definition::parse(item_impl)?;
    let init_config_descriptors = descriptor::parameters(&definition.init.parameters);
    let init_output_descriptors = descriptor::outputs(&definition.init.outputs);
    let port_probe_input_descriptors = definition
        .port_probe
        .iter()
        .flat_map(|method| descriptor::parameters(&method.parameters))
        .collect::<Vec<_>>();
    let operation_descriptors = definition.operations.iter().map(descriptor::operation);
    let init_call = dispatch::with_config(&self_type, &definition.init);
    let close_call = dispatch::with_input(&self_type, &definition.close);
    let operation_calls = definition
        .operations
        .iter()
        .map(|operation| dispatch::operation(&self_type, operation));
    let port_probe = definition
        .port_probe
        .as_ref()
        .map(|method| dispatch::port_probe(&self_type, method));
    let abi_exports = ffi::exports(&self_type, &runtime);

    Ok(quote! {
        #item_impl
        const #init_config: &[::miresidian::ParameterDescriptor] = &[#(#init_config_descriptors),*];
        const #init_outputs: &[::miresidian::ParameterDescriptor] = &[#(#init_output_descriptors),*];
        const #port_probe_inputs: &[::miresidian::ParameterDescriptor] = &[#(#port_probe_input_descriptors),*];
        const #operations: &[::miresidian::OperationDescriptor] = &[#(#operation_descriptors),*];
        const #descriptor: ::miresidian::NodeDescriptor = ::miresidian::NodeDescriptor {
            name: <#self_type>::__MIRESIDIAN_NODE_NAME, init_config: #init_config,
            init_outputs: #init_outputs, port_probe_inputs: #port_probe_inputs, operations: #operations,
        };
        impl ::miresidian::DeviceNode for #self_type {
            fn descriptor() -> &'static ::miresidian::NodeDescriptor { &#descriptor }
            fn init_device(&mut self, config: &[::std::string::String]) -> ::miresidian::DriverResult<::miresidian::InvocationOutput> { #init_call }
            fn close_device(&mut self, input: &[::std::string::String]) -> ::miresidian::DriverResult<()> { #close_call }
            fn invoke(&mut self, operation: &str, _config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::DriverResult<::miresidian::InvocationOutput> {
                match operation { #(#operation_calls)* other => Err(::miresidian::DriverError::InvalidOperation(other.to_owned())), }
            }
            #port_probe
        }
        #abi_exports
    })
}

fn type_identifier(ty: &Type) -> Result<&syn::Ident> {
    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(ty, "驱动类型必须是具名结构体"));
    };
    path.path
        .segments
        .last()
        .map(|segment| &segment.ident)
        .ok_or_else(|| Error::new_spanned(ty, "驱动类型必须是具名结构体"))
}
