//! 方法级属性解析；参数名称、来源、默认值及输出标注沿用既有规则。

use super::naming::{operation_name, parameter_name, rust_parameter_name};
use syn::{
    Attribute, Error, FnArg, GenericArgument, ImplItemFn, LitStr, PathArguments, Result,
    ReturnType, Token, Type, punctuated::Punctuated,
};

#[derive(Clone)]
pub(super) struct Parameter {
    pub(super) ty: Type,
    pub(super) name: LitStr,
    pub(super) rust_name: LitStr,
    pub(super) definition: LitStr,
    pub(super) default: Option<LitStr>,
    pub(super) source: ParameterSource,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ParameterSource {
    Config,
    Input,
}

pub(super) struct InitMethod {
    pub(super) method: syn::Ident,
    pub(super) parameters: Vec<Parameter>,
    pub(super) outputs: Vec<LitStr>,
    pub(super) has_output: bool,
}

pub(super) struct OperationMethod {
    pub(super) method: syn::Ident,
    pub(super) name: LitStr,
    pub(super) parameters: Vec<Parameter>,
    pub(super) outputs: Vec<LitStr>,
}

pub(super) struct FixedMethod {
    pub(super) method: syn::Ident,
    pub(super) parameters: Vec<Parameter>,
}

pub(super) enum NodeMethod {
    Init(InitMethod),
    Close(FixedMethod),
    PortProbe(FixedMethod),
    Operation(OperationMethod),
}

/// 单个方法即生成入口，不依赖外层 impl 扫描或宏执行顺序。
pub(super) fn parse(attribute: &Attribute, method: &mut ImplItemFn) -> Result<NodeMethod> {
    if !method.sig.generics.params.is_empty() || method.sig.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &method.sig.generics,
            "平台方法不能带泛型参数",
        ));
    }
    if method.sig.asyncness.is_some()
        || method.sig.unsafety.is_some()
        || method.sig.constness.is_some()
        || method.sig.abi.is_some()
    {
        return Err(Error::new_spanned(
            &method.sig,
            "平台方法必须是普通同步 Rust 函数",
        ));
    }
    let name = operation_name(attribute, &method.sig.ident)?;
    let identifier = method.sig.ident.clone();
    let fixed_name = match identifier.to_string().as_str() {
        "init_device" => Some("InitDevice"),
        "close_device" => Some("CloseDevice"),
        "get_port_no" => Some("GetPortNO"),
        _ => None,
    };
    if let Some(expected) = fixed_name
        && !matches!(attribute.meta, syn::Meta::Path(_))
        && name.value() != expected
    {
        return Err(Error::new_spanned(
            attribute,
            "生命周期与探测入口的平台名称固定，不接受重命名",
        ));
    }
    match identifier.to_string().as_str() {
        "init_device" => {
            require_associated_function(method)?;
            let inner = output_value_type(&method.sig.output).ok_or_else(|| {
                Error::new_spanned(
                    &method.sig.output,
                    "初始化必须返回 MiresidianResult<Self> 或 MiresidianResult<(Self, T)>",
                )
            })?;
            let (has_output, business_type): (bool, Type) = match inner {
                ty if is_self(ty) => (false, syn::parse_quote!(())),
                Type::Tuple(tuple) if tuple.elems.len() == 2 && is_self(&tuple.elems[0]) => {
                    (true, tuple.elems[1].clone())
                }
                _ => {
                    return Err(Error::new_spanned(
                        inner,
                        "初始化必须返回 MiresidianResult<Self> 或 MiresidianResult<(Self, T)>",
                    ));
                }
            };
            let return_type = syn::parse_quote!(-> MiresidianResult<#business_type>);
            let outputs = take_output(&mut method.attrs, &return_type)?;
            Ok(NodeMethod::Init(InitMethod {
                method: identifier,
                parameters: parse_parameters(method, ParameterSource::Config)?,
                outputs,
                has_output,
            }))
        }
        "close_device" => {
            require_mut_receiver(method)?;
            Ok(NodeMethod::Close(FixedMethod {
                method: identifier,
                parameters: parse_parameters(method, ParameterSource::Input)?,
            }))
        }
        "get_port_no" => {
            require_associated_function(method)?;
            Ok(NodeMethod::PortProbe(FixedMethod {
                method: identifier,
                parameters: parse_parameters(method, ParameterSource::Input)?,
            }))
        }
        _ => {
            require_mut_receiver(method)?;
            Ok(NodeMethod::Operation(OperationMethod {
                method: identifier,
                name,
                parameters: parse_parameters(method, ParameterSource::Input)?,
                outputs: take_output(&mut method.attrs, &method.sig.output)?,
            }))
        }
    }
}

fn is_self(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self"))
}

fn require_associated_function(method: &ImplItemFn) -> Result<()> {
    if method
        .sig
        .inputs
        .iter()
        .any(|argument| matches!(argument, FnArg::Receiver(_)))
    {
        return Err(Error::new_spanned(
            &method.sig,
            "init_device 和 get_port_no 必须是无 self 的关联函数",
        ));
    }
    Ok(())
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
        .filter(|argument| !matches!(argument, FnArg::Receiver(_)))
        .map(|argument| {
            let FnArg::Typed(argument) = argument else {
                return Err(Error::new_spanned(argument, "只允许一个 self 接收者"));
            };
            let config = take_attribute(&mut argument.attrs, "config");
            let rust_name = rust_parameter_name(argument)?;
            let input = take_attribute(&mut argument.attrs, "input");
            if config.is_some() && input.is_some() {
                return Err(Error::new_spanned(
                    argument,
                    "同一参数不能同时标注 #[config] 和 #[input]",
                ));
            }
            let (source, name, definition, default) = match (config, input) {
                (Some(attribute), None) => {
                    let definition = one_name(&attribute, "config")?;
                    let (name, default) = split_definition(&definition);
                    (ParameterSource::Config, name, definition, default)
                }
                (None, Some(attribute)) => {
                    let definition = one_name(&attribute, "input")?;
                    let (name, default) = split_definition(&definition);
                    (ParameterSource::Input, name, definition, default)
                }
                (None, None) => {
                    let name = parameter_name(argument)?;
                    (default_source, name.clone(), name, None)
                }
                (Some(_), Some(_)) => unreachable!("已在上方拒绝双通道标注"),
            };
            Ok(Parameter {
                ty: (*argument.ty).clone(),
                rust_name,
                name,
                definition,
                default,
                source,
            })
        })
        .collect()
}
fn contains_option_pairs(value: &str) -> bool {
    value.split('/').any(|choice| {
        let Some((actual, display)) = choice.split_once('-') else {
            return false;
        };
        !actual.trim().is_empty() && !display.trim().is_empty()
    })
}

pub(super) fn take_output(
    attributes: &mut Vec<Attribute>,
    return_type: &ReturnType,
) -> Result<Vec<LitStr>> {
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
    if segment.ident != "MiresidianResult" && segment.ident != "Result" {
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
        Some((name, suffix)) if !contains_option_pairs(suffix) => {
            (name.trim(), Some(suffix.trim()))
        }
        Some((name, _)) => (name.trim(), None),
        None => (text.trim(), None),
    };
    (
        LitStr::new(name, span),
        default.map(|default| LitStr::new(default, span)),
    )
}
