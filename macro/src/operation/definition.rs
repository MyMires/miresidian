//! 方法级属性解析；参数名称、来源、默认值及输出标注沿用既有规则。

use super::config::{self, Role};
use super::format::{self, Format};
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
    pub(super) batch: bool,
    pub(super) format: Option<Format>,
    pub(super) instance: Role,
    pub(super) lease: Role,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ParameterSource {
    Config,
    Input,
}

pub(super) struct InitMethod {
    pub(super) method: syn::Ident,
    pub(super) asynchronous: bool,
    pub(super) parameters: Vec<Parameter>,
    pub(super) outputs: Vec<LitStr>,
    pub(super) has_output: bool,
}

pub(super) struct OperationMethod {
    pub(super) method: syn::Ident,
    pub(super) asynchronous: bool,
    pub(super) name: LitStr,
    pub(super) parameters: Vec<Parameter>,
    pub(super) outputs: Vec<LitStr>,
    pub(super) output_codecs: Vec<Option<Format>>,
    pub(super) output_types: Vec<Type>,
    pub(super) output_tuple: bool,
}

pub(super) struct FixedMethod {
    pub(super) method: syn::Ident,
    pub(super) asynchronous: bool,
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
    if method.sig.unsafety.is_some() || method.sig.constness.is_some() || method.sig.abi.is_some() {
        return Err(Error::new_spanned(
            &method.sig,
            "平台方法必须是安全的 Rust 函数",
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
                asynchronous: method.sig.asyncness.is_some(),
                parameters: parse_parameters(method, ParameterSource::Config, false)?,
                outputs,
                has_output,
            }))
        }
        "close_device" => {
            require_mut_receiver(method)?;
            Ok(NodeMethod::Close(FixedMethod {
                method: identifier,
                asynchronous: method.sig.asyncness.is_some(),
                parameters: parse_parameters(method, ParameterSource::Input, false)?,
            }))
        }
        "get_port_no" => {
            require_associated_function(method)?;
            Ok(NodeMethod::PortProbe(FixedMethod {
                method: identifier,
                asynchronous: method.sig.asyncness.is_some(),
                parameters: parse_parameters(method, ParameterSource::Input, false)?,
            }))
        }
        _ => {
            require_mut_receiver(method)?;
            let parameters = parse_parameters(method, ParameterSource::Input, true)?;
            validate_batch_parameters(&parameters)?;
            let (outputs, output_codecs) =
                operation_outputs(&mut method.attrs, &method.sig.output)?;
            let platform = format::platform_parameters(&parameters, &output_codecs);
            let mut names = std::collections::HashSet::new();
            for parameter in platform
                .iter()
                .filter(|p| p.source == ParameterSource::Input)
            {
                if !names.insert(parameter.name.value()) {
                    return Err(Error::new_spanned(
                        &parameter.name,
                        "输入参数或格式选择名称重复",
                    ));
                }
            }
            let output_types = match output_value_type(&method.sig.output) {
                Some(Type::Tuple(tuple)) => tuple.elems.iter().cloned().collect(),
                Some(ty) => vec![ty.clone()],
                None => Vec::new(),
            };
            Ok(NodeMethod::Operation(OperationMethod {
                method: identifier,
                asynchronous: method.sig.asyncness.is_some(),
                name,
                parameters,
                outputs,
                output_codecs,
                output_types,
                output_tuple: matches!(output_value_type(&method.sig.output), Some(Type::Tuple(_))),
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
    batch_allowed: bool,
) -> Result<Vec<Parameter>> {
    let initialization = method.sig.ident == "init_device";
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
            if argument.attrs.iter().any(|attribute| {
                attribute
                    .path()
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "config")
            }) {
                return Err(Error::new_spanned(
                    argument,
                    "同一参数只能声明一个 config 属性",
                ));
            }
            let mut instance = Role::Default;
            let mut lease = Role::Default;
            let rust_name = rust_parameter_name(argument)?;
            let input = take_attribute(&mut argument.attrs, "input");
            if config.is_some() && input.is_some() {
                return Err(Error::new_spanned(
                    argument,
                    "同一参数不能同时标注 #[config] 和 #[input]",
                ));
            }
            let (source, name, definition, default, batch, format) = match (config, input) {
                (Some(attribute), None) => {
                    let annotation = config::annotation(&attribute, initialization)?;
                    instance = annotation.instance;
                    lease = annotation.lease;
                    let definition = annotation.definition;
                    let (name, default) = split_definition(&definition);
                    (
                        ParameterSource::Config,
                        name,
                        definition,
                        default,
                        false,
                        None,
                    )
                }
                (None, Some(attribute)) => {
                    let (mut definitions, batch, format) = format::annotation(&attribute, true)?;
                    if definitions.len() != 1 {
                        return Err(Error::new_spanned(
                            attribute,
                            "input 必须且只能声明一个名称",
                        ));
                    }
                    let definition = definitions.remove(0);
                    let (name, default) = split_definition(&definition);
                    (
                        ParameterSource::Input,
                        name,
                        definition,
                        default,
                        batch,
                        format,
                    )
                }
                (None, None) => {
                    let name = parameter_name(argument)?;
                    (default_source, name.clone(), name, None, false, None)
                }
                (Some(_), Some(_)) => unreachable!("已在上方拒绝双通道标注"),
            };
            if format.is_some() && !batch_allowed {
                return Err(Error::new_spanned(
                    argument,
                    "format 只能标注普通业务方法的 Input 或 Output",
                ));
            }
            if batch && !batch_allowed {
                return Err(Error::new_spanned(
                    argument,
                    "batch 只能标注普通业务方法的 Input 参数",
                ));
            }
            Ok(Parameter {
                ty: (*argument.ty).clone(),
                rust_name,
                name,
                definition,
                default,
                source,
                batch,
                format,
                instance,
                lease,
            })
        })
        .collect()
}

/// batch 只约束参数来源；原 Rust 类型的 codec trait 决定可接受的输入形式。
fn validate_batch_parameters(parameters: &[Parameter]) -> Result<()> {
    for parameter in parameters.iter().filter(|parameter| parameter.batch) {
        if parameter.source != ParameterSource::Input {
            return Err(Error::new_spanned(
                &parameter.ty,
                "batch 只能标注普通业务方法的 Input 参数",
            ));
        }
    }
    Ok(())
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
/// 业务输出允许逐槽位标注；旧的单标注多名称继续可用。
fn operation_outputs(
    attributes: &mut Vec<Attribute>,
    return_type: &ReturnType,
) -> Result<(Vec<LitStr>, Vec<Option<Format>>)> {
    let expected = output_count(return_type);
    let mut outputs = Vec::new();
    let mut codecs = Vec::new();
    let mut multi_names = false;
    let mut first_attribute = None;
    while let Some(attribute) = take_attribute(attributes, "output") {
        if first_attribute.is_none() {
            first_attribute = Some(attribute.clone());
        }
        let (names, _, format) = format::annotation(&attribute, false)?;
        if (!outputs.is_empty() && names.len() > 1) || multi_names {
            return Err(Error::new_spanned(
                attribute,
                "不能混用逐项输出和多名称输出标注",
            ));
        }
        multi_names = names.len() > 1;
        for name in names {
            outputs.push(name);
            codecs.push(format.clone());
        }
    }
    if outputs.is_empty() {
        outputs = default_output_names(expected);
        codecs = vec![None; expected];
    }
    if outputs.len() != expected {
        return Err(Error::new_spanned(
            first_attribute.as_ref().expect("显式输出数量不匹配"),
            format!(
                "#[output] 声明了 {} 个输出名称，但该返回类型需要 {expected} 个输出名称",
                outputs.len()
            ),
        ));
    }
    Ok((outputs, codecs))
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
    // 描述中的冒号后缀仅供平台显示，不推导业务参数的运行时默认值。
    let name = text.split_once(':').map_or(text.as_str(), |(name, _)| name);
    (LitStr::new(name.trim(), span), None)
}
