//! Miresidian 驱动声明所使用的属性过程宏。
//!
//! 驱动作者只在结构体、方法、参数和返回值处声明平台契约；本 crate 负责生成
//! `GetFunctions`、`GetInitConfig`、生命周期调度与 C ABI 所需的运行时适配代码。

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Error, FnArg, GenericArgument, ImplItem, ImplItemFn, ItemImpl, ItemStruct, LitStr,
    PathArguments, Result, ReturnType, Token, Type, parse_macro_input, punctuated::Punctuated,
};

/// 标记一个可由 Miresidian 加载的驱动结构体。
///
/// 参数是测试平台展示名称，例如 `#[miresidian::node("串口通信")]`。
#[proc_macro_attribute]
pub fn node(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let name = parse_macro_input!(attribute as LitStr);
    let item_struct = parse_macro_input!(item as ItemStruct);
    let identifier = &item_struct.ident;

    quote! {
        #item_struct

        impl #identifier {
            #[doc(hidden)]
            pub const __MIRESIDIAN_NODE_NAME: &'static str = #name;
        }
    }
    .into()
}

/// 分析一个驱动实现块并生成平台调度与 C ABI 入口。
///
/// 该实现块必须包含 `init_device` 与 `close_device` 方法。宏根据方法位置自动把
/// `init_device` 参数归入 Config、把其余方法参数归入 Input；标注只用于覆盖名称和
/// 默认值。可选的 `get_port_no` 方法将映射到固定平台操作 `GetPortNO`。
#[proc_macro_attribute]
pub fn node_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_impl = parse_macro_input!(item as ItemImpl);
    match expand_node_impl(&mut item_impl) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_node_impl(item_impl: &mut ItemImpl) -> Result<proc_macro2::TokenStream> {
    let self_type = item_impl.self_ty.as_ref();
    let self_identifier = type_identifier(self_type)?;
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

    let mut init_method = None;
    let mut close_method = None;
    let mut port_probe_method = None;
    let mut operation_methods = Vec::new();

    for item in &mut item_impl.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        if let Some(attribute) = take_attribute(&mut method.attrs, "operation") {
            let name = operation_name(&attribute, &method.sig.ident)?;
            operation_methods.push(parse_operation(method, name)?);
            continue;
        }

        match method.sig.ident.to_string().as_str() {
            "init_device" => {
                if init_method.is_some() {
                    return Err(Error::new_spanned(method, "只能声明一个 init_device 方法"));
                }
                init_method = Some(parse_init(method)?);
            }
            "close_device" => {
                if close_method.is_some() {
                    return Err(Error::new_spanned(method, "只能声明一个 close_device 方法"));
                }
                close_method = Some(parse_fixed(method)?);
            }
            "get_port_no" => {
                if port_probe_method.is_some() {
                    return Err(Error::new_spanned(method, "只能声明一个 get_port_no 方法"));
                }
                port_probe_method = Some(parse_fixed(method)?);
            }
            _ => {}
        }
    }

    let init =
        init_method.ok_or_else(|| Error::new_spanned(&*item_impl, "缺少 init_device 方法"))?;
    let close =
        close_method.ok_or_else(|| Error::new_spanned(&*item_impl, "缺少 close_device 方法"))?;
    let init_config_descriptors = parameter_descriptors(&init.parameters);
    let init_output_descriptors = output_descriptors(&init.outputs);
    let port_probe_input_descriptors = port_probe_method
        .iter()
        .flat_map(|method| parameter_descriptors(&method.parameters))
        .collect::<Vec<_>>();
    let operation_descriptors = operation_methods.iter().map(operation_descriptor);
    let init_call = call_with_config(self_type, &init.method, &init.parameters);
    let operation_calls = operation_methods
        .iter()
        .map(|operation| operation_call(self_type, operation));
    let close_call = call_with_input(self_type, &close.method, &close.parameters);
    let port_probe = port_probe_method.as_ref().map(|method| {
        let call = call_with_input(self_type, &method.method, &method.parameters);
        quote! {
            fn get_port_no(
                &mut self,
                input: &[::std::string::String],
            ) -> ::miresidian::DriverResult<::core::option::Option<::std::string::String>> {
                #call
            }
        }
    });

    Ok(quote! {
        #item_impl

        const #init_config: &[::miresidian::ParameterDescriptor] = &[
            #(#init_config_descriptors),*
        ];
        const #init_outputs: &[::miresidian::ParameterDescriptor] = &[
            #(#init_output_descriptors),*
        ];
        const #port_probe_inputs: &[::miresidian::ParameterDescriptor] = &[
            #(#port_probe_input_descriptors),*
        ];
        const #operations: &[::miresidian::OperationDescriptor] = &[
            #(#operation_descriptors),*
        ];
        const #descriptor: ::miresidian::NodeDescriptor = ::miresidian::NodeDescriptor {
            name: <#self_type>::__MIRESIDIAN_NODE_NAME,
            init_config: #init_config,
            init_outputs: #init_outputs,
            port_probe_inputs: #port_probe_inputs,
            operations: #operations,
        };

        impl ::miresidian::DeviceNode for #self_type {
            fn descriptor() -> &'static ::miresidian::NodeDescriptor {
                &#descriptor
            }

            fn init_device(
                &mut self,
                config: &[::std::string::String],
            ) -> ::miresidian::DriverResult<::miresidian::InvocationOutput> {
                #init_call
            }

            fn close_device(
                &mut self,
                input: &[::std::string::String],
            ) -> ::miresidian::DriverResult<()> {
                #close_call
            }

            fn invoke(
                &mut self,
                operation: &str,
                _config: &[::std::string::String],
                input: &[::std::string::String],
            ) -> ::miresidian::DriverResult<::miresidian::InvocationOutput> {
                match operation {
                    #(#operation_calls)*
                    other => Err(::miresidian::DriverError::InvalidOperation(other.to_owned())),
                }
            }

            #port_probe
        }

        static #runtime: ::std::sync::OnceLock<
            ::std::sync::Mutex<::miresidian::NodeRuntime<#self_type>>
        > = ::std::sync::OnceLock::new();

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_invoke_v1(
            operation: *const ::std::ffi::c_char,
            config: *const *const ::std::ffi::c_char,
            config_count: usize,
            input: *const *const ::std::ffi::c_char,
            input_count: usize,
            output: *mut ::miresidian::MiresidianOutputV1,
        ) -> ::miresidian::MiresidianStatus {
            unsafe {
                ::miresidian::ffi_runtime::invoke_v1(
                    &#runtime, operation, config, config_count, input, input_count, output,
                )
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn miresidian_free_output_v1(
            output: *mut ::miresidian::MiresidianOutputV1,
        ) {
            unsafe { ::miresidian::ffi_runtime::free_output_v1(output) }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_last_error_v1() -> *const ::std::ffi::c_char {
            ::miresidian::ffi_runtime::last_error_v1()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn miresidian_abi_version() -> u32 { 1 }
    })
}

#[derive(Clone)]
struct Parameter {
    ty: Type,
    name: LitStr,
    default: Option<LitStr>,
}

struct InitMethod {
    method: syn::Ident,
    parameters: Vec<Parameter>,
    outputs: Vec<LitStr>,
}

struct OperationMethod {
    method: syn::Ident,
    name: LitStr,
    parameters: Vec<Parameter>,
    outputs: Vec<LitStr>,
}

struct FixedMethod {
    method: syn::Ident,
    parameters: Vec<Parameter>,
}

fn parse_init(method: &mut ImplItemFn) -> Result<InitMethod> {
    require_mut_receiver(method)?;
    Ok(InitMethod {
        method: method.sig.ident.clone(),
        parameters: parse_parameters(method, "config")?,
        outputs: take_output(&mut method.attrs, &method.sig.output)?,
    })
}

fn parse_operation(method: &mut ImplItemFn, name: LitStr) -> Result<OperationMethod> {
    require_mut_receiver(method)?;
    Ok(OperationMethod {
        method: method.sig.ident.clone(),
        name,
        parameters: parse_parameters(method, "input")?,
        outputs: take_output(&mut method.attrs, &method.sig.output)?,
    })
}

fn parse_fixed(method: &mut ImplItemFn) -> Result<FixedMethod> {
    require_mut_receiver(method)?;
    Ok(FixedMethod {
        method: method.sig.ident.clone(),
        parameters: parse_parameters(method, "input")?,
    })
}

fn require_mut_receiver(method: &ImplItemFn) -> Result<()> {
    let Some(FnArg::Receiver(receiver)) = method.sig.inputs.first() else {
        return Err(Error::new_spanned(
            method,
            "方法必须以 &mut self 作为第一个参数",
        ));
    };
    if receiver.reference.is_none() || receiver.mutability.is_none() {
        return Err(Error::new_spanned(receiver, "方法接收者必须为 &mut self"));
    }
    Ok(())
}

fn parse_parameters(method: &mut ImplItemFn, attribute_name: &str) -> Result<Vec<Parameter>> {
    method
        .sig
        .inputs
        .iter_mut()
        .skip(1)
        .map(|argument| {
            let FnArg::Typed(argument) = argument else {
                return Err(Error::new_spanned(argument, "只允许一个 self 接收者"));
            };
            let (name, default) = match take_attribute(&mut argument.attrs, attribute_name) {
                Some(attribute) => split_definition(&one_name(&attribute, attribute_name)?),
                None => (parameter_name(argument)?, None),
            };
            Ok(Parameter {
                ty: (*argument.ty).clone(),
                name,
                default,
            })
        })
        .collect()
}

fn take_output(attributes: &mut Vec<Attribute>, return_type: &ReturnType) -> Result<Vec<LitStr>> {
    match take_attribute(attributes, "output") {
        Some(attribute) => names(&attribute, "output"),
        None if return_has_value(return_type) => {
            Ok(vec![LitStr::new("返回值", proc_macro2::Span::call_site())])
        }
        None => Ok(Vec::new()),
    }
}

fn parameter_descriptors(
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

fn output_descriptors(outputs: &[LitStr]) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    outputs
        .iter()
        .map(|name| quote!(::miresidian::ParameterDescriptor { name: #name, default_value: None }))
}

fn operation_descriptor(operation: &OperationMethod) -> proc_macro2::TokenStream {
    let name = &operation.name;
    let inputs = parameter_descriptors(&operation.parameters);
    let outputs = output_descriptors(&operation.outputs);
    quote! {
        ::miresidian::OperationDescriptor {
            name: #name,
            inputs: &[#(#inputs),*],
            outputs: &[#(#outputs),*],
        }
    }
}

fn call_with_config(
    self_type: &Type,
    method: &syn::Ident,
    parameters: &[Parameter],
) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(
        parameters,
        quote!(config),
        quote!(::miresidian::parse_config),
    );
    let variables = argument_variables(parameters.len());
    quote! {
        #(#arguments)*
        let value = <#self_type>::#method(self, #(#variables),*);
        ::miresidian::into_invocation_output(value?)
    }
}

fn call_with_input(
    self_type: &Type,
    method: &syn::Ident,
    parameters: &[Parameter],
) -> proc_macro2::TokenStream {
    let arguments = parse_arguments(parameters, quote!(input), quote!(::miresidian::parse_input));
    let variables = argument_variables(parameters.len());
    quote! {
        #(#arguments)*
        <#self_type>::#method(self, #(#variables),*)
    }
}

fn operation_call(self_type: &Type, operation: &OperationMethod) -> proc_macro2::TokenStream {
    let operation_name = &operation.name;
    let method = &operation.method;
    let arguments = parse_arguments(
        &operation.parameters,
        quote!(input),
        quote!(::miresidian::parse_input),
    );
    let variables = argument_variables(operation.parameters.len());
    quote! {
        #operation_name => {
            #(#arguments)*
            let value = <#self_type>::#method(self, #(#variables),*);
            ::miresidian::into_invocation_output(value?)
        }
    }
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
            quote! {
                let #variable: #ty = #parser(#values, #index, #name, #default)?;
            }
        })
}

fn argument_variables(length: usize) -> impl Iterator<Item = syn::Ident> {
    (0..length).map(|index| format_ident!("__miresidian_argument_{index}"))
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

fn parameter_name(argument: &syn::PatType) -> Result<LitStr> {
    let syn::Pat::Ident(identifier) = argument.pat.as_ref() else {
        return Err(Error::new_spanned(
            argument,
            "自动生成参数名称时，参数必须使用普通标识符",
        ));
    };
    Ok(LitStr::new(
        &identifier.ident.to_string(),
        identifier.ident.span(),
    ))
}

fn operation_name(attribute: &Attribute, method: &syn::Ident) -> Result<LitStr> {
    if matches!(attribute.meta, syn::Meta::Path(_)) {
        return Ok(platform_method_name(method));
    }
    one_name(attribute, "operation")
}

fn platform_method_name(method: &syn::Ident) -> LitStr {
    let converted = method
        .to_string()
        .split('_')
        .filter(|part| !part.is_empty())
        .map(platform_word)
        .collect::<String>();
    LitStr::new(&converted, method.span())
}

fn platform_word(word: &str) -> String {
    match word.to_ascii_lowercase().as_str() {
        "api" => "API".to_owned(),
        "com" => "COM".to_owned(),
        "dll" => "DLL".to_owned(),
        "dut" => "DUT".to_owned(),
        "id" => "ID".to_owned(),
        "io" => "IO".to_owned(),
        "ip" => "IP".to_owned(),
        "no" => "NO".to_owned(),
        "tcp" => "TCP".to_owned(),
        "udp" => "UDP".to_owned(),
        "usb" => "USB".to_owned(),
        "utf8" => "UTF8".to_owned(),
        _ => {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().chain(characters).collect(),
                None => String::new(),
            }
        }
    }
}

fn return_has_value(return_type: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = return_type else {
        return false;
    };
    let Type::Path(path) = ty.as_ref() else {
        return !is_unit_type(ty);
    };
    let Some(segment) = path.path.segments.last() else {
        return true;
    };
    if segment.ident != "DriverResult" && segment.ident != "Result" {
        return !is_unit_type(ty);
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return true;
    };
    let Some(GenericArgument::Type(inner)) = arguments.args.first() else {
        return true;
    };
    !is_unit_type(inner)
}

fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

fn take_attribute(attributes: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
    attributes
        .iter()
        .position(|attribute| {
            attribute
                .path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == name)
        })
        .map(|index| attributes.remove(index))
}

fn one_name(attribute: &Attribute, label: &str) -> Result<LitStr> {
    let mut names = names(attribute, label)?;
    if names.len() != 1 {
        return Err(Error::new_spanned(
            attribute,
            format!("#[{label}] 必须且只能声明一个名称"),
        ));
    }
    Ok(names.remove(0))
}

fn names(attribute: &Attribute, label: &str) -> Result<Vec<LitStr>> {
    let values = attribute.parse_args_with(Punctuated::<LitStr, Token![,]>::parse_terminated)?;
    if values.is_empty() {
        return Err(Error::new_spanned(
            attribute,
            format!("#[{label}] 至少需要一个名称"),
        ));
    }
    Ok(values.into_iter().collect())
}

fn split_definition(value: &LitStr) -> (LitStr, Option<LitStr>) {
    let span = value.span();
    let text = value.value();
    let (name, default) = match text.split_once(':') {
        Some((name, default)) => (name.trim(), Some(default.trim())),
        None => (text.trim(), None),
    };
    (
        LitStr::new(name, span),
        default.map(|default| LitStr::new(default, span)),
    )
}

/// 标记可由测试平台调用的驱动方法。
///
/// 仅在 `#[node_impl]` 内使用；该宏由外层实现宏消费。
#[proc_macro_attribute]
pub fn operation(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记初始化配置参数，仅在 `init_device` 的参数上使用。
#[proc_macro_attribute]
pub fn config(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记设备方法输入参数。
#[proc_macro_attribute]
pub fn input(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 标记设备方法返回值。
#[proc_macro_attribute]
pub fn output(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 自动方法名转换为平台形式() {
        assert_eq!(platform_word("port"), "Port");
        assert_eq!(platform_word("no"), "NO");
        assert_eq!(platform_word("tcp"), "TCP");

        let init = format_ident!("init_device");
        let port = format_ident!("get_port_no");
        let read = format_ident!("read_text");
        assert_eq!(platform_method_name(&init).value(), "InitDevice");
        assert_eq!(platform_method_name(&port).value(), "GetPortNO");
        assert_eq!(platform_method_name(&read).value(), "ReadText");
    }
}
