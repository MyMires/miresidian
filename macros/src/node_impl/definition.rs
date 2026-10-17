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
    pub(super) source: ParameterSource,
}

/// 参数应从外部 `invoke` 的哪个数组读取。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ParameterSource {
    /// 从 `config` 数组读取。
    Config,
    /// 从 `input` 数组读取。
    Input,
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
    if item_impl.trait_.is_some() {
        return Err(Error::new_spanned(
            &*item_impl,
            "#[node_impl] 只能标注在节点结构体的固有 impl 块上，不能标注 trait impl",
        ));
    }
    if !item_impl.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &item_impl.generics,
            "#[node_impl] 暂不支持带泛型参数的 impl；请为每个具体节点类型声明独立 impl",
        ));
    }
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
        parameters: parse_parameters(method, ParameterSource::Config)?,
        outputs: take_output(&mut method.attrs, &method.sig.output)?,
    })
}
fn parse_operation(method: &mut ImplItemFn, name: LitStr) -> Result<OperationMethod> {
    require_mut_receiver(method)?;
    Ok(OperationMethod {
        method: method.sig.ident.clone(),
        name,
        parameters: parse_parameters(method, ParameterSource::Input)?,
        outputs: take_output(&mut method.attrs, &method.sig.output)?,
    })
}
fn parse_fixed(method: &mut ImplItemFn) -> Result<FixedMethod> {
    require_mut_receiver(method)?;
    Ok(FixedMethod {
        method: method.sig.ident.clone(),
        parameters: parse_parameters(method, ParameterSource::Input)?,
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
/// 解析方法参数。未标注参数采用方法类别的默认来源；`config` 与 `input` 可逐参数覆盖。
fn parse_parameters(
    method: &mut ImplItemFn,
    default_source: ParameterSource,
) -> Result<Vec<Parameter>> {
    method
        .sig
        .inputs
        .iter_mut()
        .skip(1)
        .map(|argument| {
            let FnArg::Typed(argument) = argument else {
                return Err(Error::new_spanned(argument, "只允许一个 self 接收者"));
            };
            let config = take_attribute(&mut argument.attrs, "config");
            let input = take_attribute(&mut argument.attrs, "input");
            if config.is_some() && input.is_some() {
                return Err(Error::new_spanned(
                    argument,
                    "同一参数不能同时标注 #[config] 和 #[input]",
                ));
            }
            let (source, name, default) = match (config, input) {
                (Some(attribute), None) => {
                    let (name, default) = split_definition(&one_name(&attribute, "config")?);
                    (ParameterSource::Config, name, default)
                }
                (None, Some(attribute)) => {
                    let (name, default) = split_definition(&one_name(&attribute, "input")?);
                    (ParameterSource::Input, name, default)
                }
                (None, None) => {
                    let (name, default) = (parameter_name(argument)?, None);
                    (default_source, name, default)
                }
                (Some(_), Some(_)) => unreachable!("已在上方拒绝双通道标注"),
            };
            Ok(Parameter {
                ty: (*argument.ty).clone(),
                name,
                default,
                source,
            })
        })
        .collect()
}
fn take_output(attributes: &mut Vec<Attribute>, return_type: &ReturnType) -> Result<Vec<LitStr>> {
    let expected_count = output_count(return_type);
    match take_attribute(attributes, "output") {
        Some(attribute) => {
            let outputs = names(&attribute, "output")?;
            if outputs.len() != expected_count {
                return Err(Error::new_spanned(
                    attribute,
                    format!(
                        "#[output] 声明了 {} 个输出名称，但该返回类型需要 {expected_count} 个输出名称",
                        outputs.len()
                    ),
                ));
            }
            Ok(outputs)
        }
        None => Ok(default_output_names(expected_count)),
    }
}
/// 返回类型会映射成多少个平台输出。
///
/// 标量、字符串与数组占一个输出；元组的每个元素分别占一个输出；`()` 不产生输出。
fn output_count(return_type: &ReturnType) -> usize {
    output_value_type(return_type).map_or(0, |ty| match ty {
        Type::Tuple(tuple) => tuple.elems.len(),
        _ => 1,
    })
}

fn output_value_type(return_type: &ReturnType) -> Option<&Type> {
    let ReturnType::Type(_, ty) = return_type else {
        return None;
    };
    let Type::Path(path) = ty.as_ref() else {
        return Some(ty);
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "DriverResult" && segment.ident != "Result" {
        return Some(ty);
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Some(ty);
    };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    })
}

fn default_output_names(count: usize) -> Vec<LitStr> {
    match count {
        0 => Vec::new(),
        1 => vec![LitStr::new("返回值", proc_macro2::Span::call_site())],
        count => (1..=count)
            .map(|index| LitStr::new(&format!("返回值{index}"), proc_macro2::Span::call_site()))
            .collect(),
    }
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
