//! `#[node_impl]` 的编排层。
//!
//! 只负责串联解析、描述信息、调度和 C ABI 生成；具体规则按职责位于同名目录下。

mod definition;
mod descriptor;
mod dispatch;
mod ffi;
mod naming;

use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};
use syn::{Error, ItemImpl, Result, Type, visit_mut::VisitMut};

fn rewrite_framework_paths(tokens: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
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

/// 分析节点实现块并生成平台调度与 C ABI 入口。
pub(crate) fn expand_node_impl(item_impl: &mut ItemImpl) -> Result<proc_macro2::TokenStream> {
    let self_type = (*item_impl.self_ty).clone();
    let self_identifier = type_identifier(&self_type)?;
    let prefix = format_ident!(
        "__MIRESIDIAN_{}",
        self_identifier.to_string().to_uppercase()
    );
    let init_config = format_ident!("{prefix}_INIT_CONFIG");
    let init_inputs = format_ident!("{prefix}_INIT_INPUTS");
    let init_outputs = format_ident!("{prefix}_INIT_OUTPUTS");
    let port_probe_inputs = format_ident!("{prefix}_PORT_PROBE_INPUTS");
    let operations = format_ident!("{prefix}_OPERATIONS");
    let descriptor = format_ident!("{prefix}_DESCRIPTOR");
    let runtime = format_ident!("{prefix}_RUNTIME");

    let definition = definition::parse(item_impl)?;
    // `InitDevice` 可以逐参数改从 Input 读取。只有 Config 来源的参数才是
    // 通道身份的一部分；运行时据此剥离租约和共享执行方式等框架保留配置。
    let init_config_parameters = definition
        .init
        .parameters
        .iter()
        .filter(|parameter| parameter.source == definition::ParameterSource::Config)
        .cloned()
        .collect::<Vec<_>>();
    let init_config_descriptors = descriptor::parameters(&init_config_parameters);
    let init_input_parameters = definition
        .init
        .parameters
        .iter()
        .filter(|parameter| parameter.source == definition::ParameterSource::Input)
        .cloned()
        .collect::<Vec<_>>();
    let init_input_descriptors = descriptor::parameters(&init_input_parameters);
    let init_output_descriptors = descriptor::outputs(&definition.init.outputs);
    let port_probe_input_descriptors = definition
        .port_probe
        .iter()
        .flat_map(|method| descriptor::parameters(&method.parameters))
        .collect::<Vec<_>>();
    let operation_descriptors = definition.operations.iter().map(descriptor::operation);
    let init_call = dispatch::with_config(&self_type, &definition.init);
    let close_call = dispatch::close_with_lease(&self_type, &definition.close);
    let lease_parameters = dispatch::lease_parameters(&definition.init);
    let operation_calls = definition
        .operations
        .iter()
        .map(|operation| dispatch::operation(&self_type, operation));
    let port_probe = definition
        .port_probe
        .as_ref()
        .map(|method| dispatch::port_probe(&self_type, method));
    let abi_exports = ffi::exports(&self_type, &runtime);

    let expanded = quote! {
        #item_impl
        const #init_config: &[::miresidian::ParameterDescriptor] = &[#(#init_config_descriptors),*];
        const #init_inputs: &[::miresidian::ParameterDescriptor] = &[#(#init_input_descriptors),*];
        const #init_outputs: &[::miresidian::ParameterDescriptor] = &[#(#init_output_descriptors),*];
        const #port_probe_inputs: &[::miresidian::ParameterDescriptor] = &[#(#port_probe_input_descriptors),*];
        const #operations: &[::miresidian::OperationDescriptor] = &[#(#operation_descriptors),*];
        const #descriptor: ::miresidian::NodeDescriptor = ::miresidian::NodeDescriptor {
            name: <#self_type>::__MIRESIDIAN_NODE_NAME, init_config: #init_config,
            init_inputs: #init_inputs, init_outputs: #init_outputs, port_probe_inputs: #port_probe_inputs, operations: #operations, lease_activation: <#self_type>::__MIRESIDIAN_LEASE_ACTIVATION,
        };
        impl #self_type {
            #[doc(hidden)]
            fn __miresidian_lease_parameters(config: &[::std::string::String]) -> ::miresidian::MiresidianResult<::std::vec::Vec<(::std::string::String, ::std::string::String)>> {
                #lease_parameters
            }
        }
        impl ::miresidian::DeviceNode for #self_type {
            fn descriptor() -> &'static ::miresidian::NodeDescriptor { &#descriptor }
            fn init_device(&mut self, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<::miresidian::InvocationOutput> { #init_call }
            fn lease_request(&self, config: &[::std::string::String]) -> ::miresidian::MiresidianResult<::core::option::Option<::miresidian::lease::LeaseRequest>> { <#self_type>::__miresidian_lease_request(config) }
            fn close_device(&mut self, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<()> { #close_call }
            fn project_version() -> &'static str { env!("CARGO_PKG_VERSION") }
            fn invoke(&mut self, operation: &str, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<::miresidian::InvocationOutput> {
                match operation { #(#operation_calls)* other => Err(::miresidian::MiresidianError::InvalidOperation(other.to_owned())), }
            }
            #port_probe
        }
        #abi_exports
    };
    rewrite_framework_paths(expanded)
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
