//! 节点实现中的属性解析、签名校验与内部定义模型。

use super::naming::{operation_name, parameter_name};
use syn::{
    Attribute, Error, FnArg, GenericArgument, ImplItem, ImplItemFn, ItemImpl, LitStr,
    PathArguments, Result, ReturnType, Token, Type, punctuated::Punctuated,
};

/// 一个 `#[node_impl]` 块解析完成后的完整声明模型。
///
/// 后续代码生成只读取该模型，因此解析阶段与描述、调度、ABI 生成阶段不会互相耦合。
pub(super) struct NodeDefinition {
    pub(super) init: InitMethod,
    pub(super) close: FixedMethod,
    pub(super) port_probe: Option<FixedMethod>,
    pub(super) operations: Vec<OperationMethod>,
}
/// 单个配置或输入参数的 Rust 类型与平台声明。
#[derive(Clone)]
pub(super) struct Parameter {
    pub(super) ty: Type,
    pub(super) name: LitStr,
    pub(super) default: Option<LitStr>,
}
/// 固定的初始化方法；其参数在平台侧属于 Config。
pub(super) struct InitMethod {
    pub(super) method: syn::Ident,
    pub(super) parameters: Vec<Parameter>,
    pub(super) outputs: Vec<LitStr>,
}
/// 由 `#[operation]` 声明的普通平台操作。
pub(super) struct OperationMethod {
    pub(super) method: syn::Ident,
    pub(super) name: LitStr,
    pub(super) parameters: Vec<Parameter>,
    pub(super) outputs: Vec<LitStr>,
}
/// 固定入口方法；其参数在平台侧属于 Input。
pub(super) struct FixedMethod {
    pub(super) method: syn::Ident,
    pub(super) parameters: Vec<Parameter>,
}

/// 从 `impl` 块提取各类节点方法，并移除仅供编译期使用的属性。
pub(super) fn parse(item_impl: &mut ItemImpl) -> Result<NodeDefinition> {
    let mut init_method = None;
    let mut close_method = None;
    let mut port_probe_method = None;
    let mut operation_methods = Vec::new();
    for item in &mut item_impl.items {
        let ImplItem::Fn(method) = item else { continue };
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
    Ok(NodeDefinition {
        init: init_method
            .ok_or_else(|| Error::new_spanned(&*item_impl, "缺少 init_device 方法"))?,
        close: close_method
            .ok_or_else(|| Error::new_spanned(&*item_impl, "缺少 close_device 方法"))?,
        port_probe: port_probe_method,
        operations: operation_methods,
    })
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
    let mut values = names(attribute, label)?;
    if values.len() != 1 {
        return Err(Error::new_spanned(
            attribute,
            format!("#[{label}] 必须且只能声明一个名称"),
        ));
    }
    Ok(values.remove(0))
}
pub(super) fn names(attribute: &Attribute, label: &str) -> Result<Vec<LitStr>> {
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
