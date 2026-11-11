//! 租约长短声明的纯解析；复用已有激活策略和字段校验。

use quote::quote;
use syn::{
    Error, Expr, ExprArray, ExprLit, Lit, LitStr, Meta, Result, Token, punctuated::Punctuated,
};

pub(super) struct LeaseDeclaration {
    pub(super) resource: Option<LitStr>,
    pub(super) configuration: Option<Vec<LitStr>>,
    pub(super) activation: proc_macro2::TokenStream,
}

/// 裸 lease 固定使用租约，不隐含平台选择器。
pub(super) fn required() -> LeaseDeclaration {
    LeaseDeclaration {
        resource: None,
        configuration: None,
        activation: quote!(::miresidian::LeaseActivation::Required),
    }
}

pub(super) fn parse(values: Punctuated<Meta, Token![,]>) -> Result<LeaseDeclaration> {
    if let Some(first) = values.first()
        && !matches!(first, Meta::NameValue(_))
    {
        if values.len() != 1 {
            return Err(Error::new_spanned(
                values,
                "lease 短写只能声明一个策略，不能混用长写字段",
            ));
        }
        let activation = match first {
            Meta::Path(path) if path.is_ident("required") => {
                quote!(::miresidian::LeaseActivation::Required)
            }
            Meta::Path(path) if path.is_ident("disabled") => {
                quote!(::miresidian::LeaseActivation::Disabled)
            }
            Meta::List(list) if list.path.is_ident("selectable") => {
                let defaults =
                    list.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)?;
                if defaults.len() != 1 || !defaults[0].is_ident("required") {
                    return Err(Error::new_spanned(
                        list,
                        "租约可选策略须写为 selectable(required)",
                    ));
                }
                quote!(::miresidian::LeaseActivation::Selectable)
            }
            _ => {
                return Err(Error::new_spanned(
                    first,
                    "lease 策略只支持 required、disabled、selectable(required)",
                ));
            }
        };
        return Ok(LeaseDeclaration {
            activation,
            ..required()
        });
    }
    parse_long(values)
}

fn parse_long(values: Punctuated<Meta, Token![,]>) -> Result<LeaseDeclaration> {
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
    if resource.is_some() != configuration.is_some() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "旧 lease 的 resource 和 configuration 必须成对声明",
        ));
    }
    Ok(LeaseDeclaration {
        activation: activation
            .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "lease 缺少 activation"))?,
        resource,
        configuration,
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

#[cfg(test)]
mod tests {
    use super::parse;
    use syn::parse::Parser;

    fn declaration(source: &str) -> syn::Result<super::LeaseDeclaration> {
        syn::punctuated::Punctuated::parse_terminated
            .parse_str(source)
            .and_then(parse)
    }

    #[test]
    fn all_short_policies_preserve_legacy_activation() {
        for (short, long) in [
            ("required", "activation = required"),
            ("disabled", "activation = disabled"),
            ("selectable(required)", "activation = selectable"),
        ] {
            let short = declaration(short).unwrap();
            let long = declaration(long).unwrap();
            assert_eq!(short.activation.to_string(), long.activation.to_string());
            assert!(short.resource.is_none());
            assert!(short.configuration.is_none());
        }
    }

    #[test]
    fn legacy_field_lists_and_pairing_are_preserved() {
        let old =
            declaration("activation = required, resource = \"port\", configuration = [\"speed\"]")
                .unwrap();
        assert_eq!(old.resource.unwrap().value(), "port");
        assert_eq!(old.configuration.unwrap()[0].value(), "speed");
        for source in [
            "",
            "selectable",
            "selectable()",
            "selectable(disabled)",
            "selectable(required, disabled)",
            "required, disabled",
            "required, resource = \"port\", configuration = [\"speed\"]",
            "activation = required, disabled",
            "activation = required, resource = \"port\"",
            "activation = required, configuration = [\"speed\"]",
            "activation = required, resource = \"port\", configuration = []",
            "activation = required, activation = selectable",
        ] {
            assert!(declaration(source).is_err(), "{source}");
        }
    }
}
