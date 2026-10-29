//! 节点身份、平台适配器与租约声明的解析。

use quote::quote;
use syn::{
    Error, Expr, ExprArray, ExprLit, Ident, Lit, LitStr, Meta, Path, Result, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub(super) struct LeaseDeclaration {
    pub(super) resource: LitStr,
    pub(super) configuration: Vec<LitStr>,
    pub(super) activation: proc_macro2::TokenStream,
}

pub(super) struct NodeArguments {
    pub(super) name: LitStr,
    pub(super) lease: Option<LeaseDeclaration>,
    pub(super) platforms: Vec<Path>,
    pub(super) instances: Option<proc_macro2::TokenStream>,
}

impl Parse for NodeArguments {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        let mut lease = None;
        let mut platforms = None;
        let mut instances = None;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let keyword: Ident = input.parse()?;
            let content;
            parenthesized!(content in input);
            match keyword.to_string().as_str() {
                "instances" => {
                    if instances.is_some() {
                        return Err(Error::new_spanned(keyword, "instances 只能声明一次"));
                    }
                    let values = content.parse_terminated(Meta::parse, Token![,])?;
                    instances = Some(super::instances::parse(values)?);
                }
                "lease" => {
                    if lease.is_some() {
                        return Err(Error::new_spanned(keyword, "lease 只能声明一次"));
                    }
                    let values = content.parse_terminated(Meta::parse, Token![,])?;
                    lease = Some(parse_lease_declaration(values)?);
                }
                "platforms" => {
                    if platforms.is_some() {
                        return Err(Error::new_spanned(keyword, "platforms 只能声明一次"));
                    }
                    let values = content.parse_terminated(Path::parse_mod_style, Token![,])?;
                    if values.is_empty() {
                        return Err(Error::new_spanned(
                            keyword,
                            "platforms 至少需要一个适配器类型",
                        ));
                    }
                    platforms = Some(values.into_iter().collect());
                }
                _ => {
                    return Err(Error::new_spanned(
                        keyword,
                        "node 子配置仅支持 lease(...)、platforms(...) 和 instances(...)",
                    ));
                }
            }
        }
        Ok(Self {
            name,
            lease,
            platforms: platforms.unwrap_or_default(),
            instances,
        })
    }
}
fn parse_lease_declaration(values: Punctuated<Meta, Token![,]>) -> Result<LeaseDeclaration> {
    let mut resource = None;
    let mut configuration = None;
    let mut activation = None;
    for value in values {
        let Meta::NameValue(value) = value else {
            return Err(Error::new_spanned(
                value,
                "lease 参数必须使用 resource = 字段 或 configuration = [字段, ...]",
            ));
        };
        let key = value
            .path
            .get_ident()
            .map(ToString::to_string)
            .ok_or_else(|| Error::new_spanned(&value.path, "lease 参数名称必须是标识符"))?;
        match key.as_str() {
            "resource" => {
                if resource.is_some() {
                    return Err(Error::new_spanned(value, "resource 只能声明一次"));
                }
                resource = Some(string_literal(&value.value, "resource")?);
            }
            "configuration" => {
                if configuration.is_some() {
                    return Err(Error::new_spanned(value, "configuration 只能声明一次"));
                }
                let Expr::Array(ExprArray { ref elems, .. }) = value.value else {
                    return Err(Error::new_spanned(value, "configuration 必须是字符串数组"));
                };
                let values = elems
                    .iter()
                    .map(|value| string_literal(value, "configuration"))
                    .collect::<Result<Vec<_>>>()?;
                if values.is_empty() {
                    return Err(Error::new_spanned(value, "configuration 至少需要一个字段"));
                }
                configuration = Some(values);
            }
            "activation" => {
                if activation.is_some() {
                    return Err(Error::new_spanned(value, "activation 只能声明一次"));
                }
                activation = Some(activation_value(&value.value)?);
            }
            _ => {
                return Err(Error::new_spanned(
                    value,
                    "lease 只接受 activation、resource 和 configuration 参数",
                ));
            }
        }
    }
    Ok(LeaseDeclaration {
        activation: activation
            .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "lease 缺少 activation"))?,
        resource: resource
            .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "lease 缺少 resource"))?,
        configuration: configuration.ok_or_else(|| {
            Error::new(proc_macro2::Span::call_site(), "lease 缺少 configuration")
        })?,
    })
}

fn string_literal(value: &Expr, label: &str) -> Result<LitStr> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = value
    else {
        return Err(Error::new_spanned(
            value,
            format!("{label} 字段必须是字符串"),
        ));
    };
    Ok(value.clone())
}

fn activation_value(value: &Expr) -> Result<proc_macro2::TokenStream> {
    let Expr::Path(path) = value else {
        return Err(Error::new_spanned(
            value,
            "activation 必须是 required、disabled 或 selectable",
        ));
    };
    let Some(name) = path.path.get_ident() else {
        return Err(Error::new_spanned(
            value,
            "activation 必须是 required、disabled 或 selectable",
        ));
    };
    match name.to_string().as_str() {
        "required" => Ok(quote!(::miresidian::LeaseActivation::Required)),
        "disabled" => Ok(quote!(::miresidian::LeaseActivation::Disabled)),
        "selectable" => Ok(quote!(::miresidian::LeaseActivation::Selectable)),
        _ => Err(Error::new_spanned(
            value,
            "activation 必须是 required、disabled 或 selectable",
        )),
    }
}
