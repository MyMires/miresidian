//! 节点级代码生成；方法契约由各个 operation 宏独立提交。

use quote::{format_ident, quote};
use syn::ItemStruct;

use super::{arguments::NodeArguments, ffi};

pub(super) fn expand(item: &ItemStruct, arguments: &NodeArguments) -> proc_macro2::TokenStream {
    let identifier = &item.ident;
    let name = &arguments.name;
    let platforms = &arguments.platforms;
    let runtime = format_ident!(
        "__MIRESIDIAN_{}_RUNTIME",
        identifier.to_string().to_uppercase()
    );
    let self_type = syn::parse_quote!(#identifier);
    let exports = ffi::exports(&self_type, &runtime);
    let instances = arguments
        .instances
        .clone()
        .unwrap_or_else(|| quote!(::miresidian::InstancePolicy::DEFAULT));
    let activation = arguments
        .lease
        .as_ref()
        .map(|lease| lease.activation.clone())
        .unwrap_or_else(|| quote!(::miresidian::LeaseActivation::Disabled));
    let lease_request = match &arguments.lease {
        Some(lease) => {
            let resource = &lease.resource;
            let configuration = &lease.configuration;
            quote! {
                ::miresidian::__lease_feature_required();
                let values = Self::__miresidian_lease_parameters(config)?;
                let parameters = values.into_iter().map(|(name, value)| ::miresidian::lease::LeaseParameter::new(name, value)).collect();
                let request = ::miresidian::lease::LeaseRequest::from_parameters(parameters, #resource, &[#(#configuration),*])
                    .map_err(|error| ::miresidian::MiresidianError::Failure(error.to_string()))?;
                Ok(Some(request))
            }
        }
        None => quote!(let _ = config; Ok(None)),
    };

    quote! {
        #item
        const _: () = {
            let policy = #instances;
            if matches!(policy.mode, ::miresidian::InstanceMode::Multiple)
                && matches!(policy.binding, ::miresidian::InstanceBinding::Config)
            {
                assert!(#identifier::__MIRESIDIAN_INIT_CONFIG.len() > 0,
                    "Config 关联的多实例节点必须在 Config[0] 声明业务主键");
            }
        };
        // Node and its platform methods share a module. The alias binds independently
        // expanded method macros without reading source files or sharing compiler state.
        type __MiresidianNode = #identifier;
        struct __MiresidianOperation {
            descriptor: ::miresidian::OperationDescriptor,
            call_async: Option<for<'a> fn(&'a mut __MiresidianNode,&'a [::std::string::String],&'a [::std::string::String])->::miresidian::NodeFuture<'a>>,
            call: fn(&mut __MiresidianNode, &[::std::string::String], &[::std::string::String]) -> ::miresidian::MiresidianResult<::miresidian::InvocationOutput>,
            source: (&'static str, u32, u32),
        }
        struct __MiresidianPortProbe {
            inputs: &'static [::miresidian::ParameterDescriptor],
            call: fn(&[::std::string::String], &[::std::string::String]) -> ::miresidian::MiresidianResult<::core::option::Option<::std::string::String>>,
        }
        ::miresidian::__private::inventory::collect!(__MiresidianOperation);
        ::miresidian::__private::inventory::collect!(__MiresidianPortProbe);

        impl #identifier {
            fn __miresidian_operations() -> &'static [&'static __MiresidianOperation] {
                static OPERATIONS: ::std::sync::OnceLock<::std::vec::Vec<&'static __MiresidianOperation>> = ::std::sync::OnceLock::new();
                OPERATIONS.get_or_init(|| {
                    let mut operations = ::miresidian::__private::inventory::iter::<__MiresidianOperation>.into_iter().collect::<::std::vec::Vec<_>>();
                    operations.sort_by_key(|entry| (entry.source, entry.descriptor.name));
                    operations
                })
            }
        }

        impl ::miresidian::DeviceNode for #identifier {
            fn descriptor() -> &'static ::miresidian::NodeDescriptor {
                static OPERATIONS: ::std::sync::OnceLock<::std::vec::Vec<::miresidian::OperationDescriptor>> = ::std::sync::OnceLock::new();
                static DESCRIPTOR: ::std::sync::OnceLock<::miresidian::NodeDescriptor> = ::std::sync::OnceLock::new();
                DESCRIPTOR.get_or_init(|| ::miresidian::NodeDescriptor {
                    name: #name,
                    init_config: Self::__MIRESIDIAN_INIT_CONFIG,
                    init_inputs: Self::__MIRESIDIAN_INIT_INPUTS,
                    init_outputs: Self::__MIRESIDIAN_INIT_OUTPUTS,
                    port_probe_inputs: ::miresidian::__private::inventory::iter::<__MiresidianPortProbe>.into_iter().next().map_or(&[], |probe| probe.inputs),
                    operations: OPERATIONS.get_or_init(|| Self::__miresidian_operations().iter().map(|entry| entry.descriptor).collect()),
                    lease_activation: #activation,
                    instances: #instances,
                })
            }

            fn validate_definition() -> ::miresidian::MiresidianResult<()> {
                static VALIDATION: ::std::sync::OnceLock<::miresidian::MiresidianResult<()>> = ::std::sync::OnceLock::new();
                VALIDATION.get_or_init(|| ::miresidian::__private::validate_descriptor(Self::descriptor())).clone()
            }

            fn platform_registry() -> ::miresidian::PlatformRegistry {
                #[allow(unused_mut)]
                let mut registry = ::miresidian::PlatformRegistry::default();
                #(registry.register(<#platforms as ::core::default::Default>::default());)*
                registry
            }

            fn init_device(config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<(Self, ::miresidian::InvocationOutput)> {
                Self::__miresidian_initialize(config, input)
            }

            fn validate_init(config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<()> {
                Self::__miresidian_validate_init(config, input)
            }

            fn lease_request(config: &[::std::string::String]) -> ::miresidian::MiresidianResult<::core::option::Option<::miresidian::lease::LeaseRequest>> {
                #lease_request
            }

            fn close_device(&mut self, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<()> {
                Self::__miresidian_close(self, config, input)
            }

            fn project_version() -> &'static str { env!("CARGO_PKG_VERSION") }

            fn invoke(&mut self, operation: &str, config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<::miresidian::InvocationOutput> {
                match Self::__miresidian_operations().iter().find(|entry| entry.descriptor.name == operation) {
                    Some(entry) => (entry.call)(self, config, input),
                    None => Err(::miresidian::MiresidianError::InvalidOperation(operation.to_owned())),
                }
            }

            fn is_async_operation(operation:&str)->bool {
                Self::__miresidian_operations().iter().any(|entry|entry.descriptor.name==operation && entry.call_async.is_some())
            }
            fn invoke_async<'a>(&'a mut self,operation:&'a str,config:&'a [::std::string::String],input:&'a [::std::string::String])->::miresidian::NodeFuture<'a> {
                match Self::__miresidian_operations().iter().find(|entry|entry.descriptor.name==operation).and_then(|entry|entry.call_async) {
                    Some(call)=>call(self,config,input),None=>Box::pin(async move {<Self as ::miresidian::DeviceNode>::invoke(self,operation,config,input)})
                }
            }

            fn get_port_no(config: &[::std::string::String], input: &[::std::string::String]) -> ::miresidian::MiresidianResult<::core::option::Option<::std::string::String>> {
                match ::miresidian::__private::inventory::iter::<__MiresidianPortProbe>.into_iter().next() {
                    Some(probe) => (probe.call)(config, input),
                    None => Ok(None),
                }
            }
        }
        #exports
    }
}
