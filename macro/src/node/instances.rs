//! instances 子配置的编译期约束与策略生成。

use quote::quote;
use std::collections::HashMap;
use syn::{Error, Expr, Meta, Result, Token, punctuated::Punctuated};

pub(super) fn parse(values: Punctuated<Meta, Token![,]>) -> Result<proc_macro2::TokenStream> {
    if values
        .iter()
        .any(|value| matches!(value, Meta::NameValue(_)))
    {
        // 旧长写仍严格校验必填项，不从短写默认规则补值。
        if !values
            .iter()
            .all(|value| matches!(value, Meta::NameValue(_)))
        {
            return Err(Error::new_spanned(
                values,
                "instances 不能混用长写字段与位置参数",
            ));
        }
        parse_long(values)
    } else {
        parse_short(values)
    }
}

fn parse_long(values: Punctuated<Meta, Token![,]>) -> Result<proc_macro2::TokenStream> {
    let mut fields = HashMap::new();
    for value in values {
        let Meta::NameValue(value) = value else {
            return Err(Error::new_spanned(
                value,
                "instances 参数必须使用 名称 = 值",
            ));
        };
        let name = value
            .path
            .get_ident()
            .map(ToString::to_string)
            .ok_or_else(|| Error::new_spanned(&value.path, "instances 参数名称必须是标识符"))?;
        if !matches!(
            name.as_str(),
            "mode" | "binding" | "execution" | "default_execution"
        ) {
            return Err(Error::new_spanned(
                value,
                "instances 只接受 mode、binding、execution 和 default_execution",
            ));
        }
        if fields.insert(name, value.value.clone()).is_some() {
            return Err(Error::new_spanned(value, "instances 参数不能重复声明"));
        }
    }
    let mode = required(&fields, "mode")?;
    let binding = fields.get("binding");
    let (mode, binding) = match ident(mode)?.as_str() {
        "single" => {
            if let Some(binding) = binding {
                return Err(Error::new_spanned(binding, "single 模式不接受 binding"));
            }
            (
                quote!(::miresidian::InstanceMode::Single),
                quote!(::miresidian::InstanceBinding::Config),
            )
        }
        "multiple" => {
            let binding = required(&fields, "binding")?;
            let binding = match ident(binding)?.as_str() {
                "config" => quote!(::miresidian::InstanceBinding::Config),
                "handle" => quote!(::miresidian::InstanceBinding::Handle),
                _ => {
                    return Err(Error::new_spanned(
                        binding,
                        "binding 必须是 config 或 handle",
                    ));
                }
            };
            (quote!(::miresidian::InstanceMode::Multiple), binding)
        }
        _ => return Err(Error::new_spanned(mode, "mode 必须是 single 或 multiple")),
    };
    let execution = required(&fields, "execution")?;
    let execution = if ident(execution)? == "selectable" {
        let default = execution_mode(required(&fields, "default_execution")?)?;
        quote!(::miresidian::ExecutionPolicy::Selectable(#default))
    } else {
        if let Some(default) = fields.get("default_execution") {
            return Err(Error::new_spanned(
                default,
                "固定 execution 不接受 default_execution",
            ));
        }
        let execution = execution_mode(execution)?;
        quote!(::miresidian::ExecutionPolicy::Fixed(#execution))
    };
    Ok(policy(mode, binding, execution))
}

/// 省略值只在新短写中生效；模式、关联和策略按互斥值域识别。
fn parse_short(values: Punctuated<Meta, Token![,]>) -> Result<proc_macro2::TokenStream> {
    let mut mode = None;
    let mut binding = None;
    let mut execution = None;
    for value in &values {
        match value {
            Meta::Path(path) if path.is_ident("single") || path.is_ident("multiple") => {
                if mode.replace(path).is_some() {
                    return Err(Error::new_spanned(value, "instances 模式不能重复声明"));
                }
            }
            Meta::Path(path) if path.is_ident("config") || path.is_ident("handle") => {
                if binding.replace(path).is_some() {
                    return Err(Error::new_spanned(value, "instances 关联方式不能重复声明"));
                }
            }
            Meta::Path(path)
                if path.is_ident("sequential")
                    || path.is_ident("parallel")
                    || path.is_ident("broadcast") =>
            {
                if execution.is_some() {
                    return Err(Error::new_spanned(value, "instances 执行策略不能重复声明"));
                }
                let expression = syn::parse_quote!(#path);
                let mode = execution_mode(&expression)?;
                execution = Some(quote!(::miresidian::ExecutionPolicy::Fixed(#mode)));
            }
            Meta::List(list) if list.path.is_ident("selectable") => {
                if execution.is_some() {
                    return Err(Error::new_spanned(value, "instances 执行策略不能重复声明"));
                }
                let defaults =
                    list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
                if defaults.len() != 1 {
                    return Err(Error::new_spanned(
                        list,
                        "selectable 必须指定一个默认执行方式",
                    ));
                }
                let default = execution_mode(&defaults[0])?;
                execution = Some(quote!(::miresidian::ExecutionPolicy::Selectable(#default)));
            }
            _ => {
                return Err(Error::new_spanned(
                    value,
                    "instances 短写只支持 single/multiple、config/handle 和执行策略",
                ));
            }
        }
    }
    let multiple = mode.is_some_and(|mode| mode.is_ident("multiple"));
    if multiple && binding.is_none() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "multiple 模式必须指定 config 或 handle",
        ));
    }
    if !multiple && let Some(binding) = binding {
        return Err(Error::new_spanned(
            binding,
            "single 模式不接受 binding；多实例须显式声明 multiple",
        ));
    }
    let mode = if multiple {
        quote!(::miresidian::InstanceMode::Multiple)
    } else {
        quote!(::miresidian::InstanceMode::Single)
    };
    let binding = if binding.is_some_and(|binding| binding.is_ident("handle")) {
        quote!(::miresidian::InstanceBinding::Handle)
    } else {
        quote!(::miresidian::InstanceBinding::Config)
    };
    let execution = execution.unwrap_or_else(|| {
        quote!(::miresidian::ExecutionPolicy::Fixed(
            ::miresidian::ExecutionMode::Sequential
        ))
    });
    Ok(policy(mode, binding, execution))
}

fn policy(
    mode: proc_macro2::TokenStream,
    binding: proc_macro2::TokenStream,
    execution: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote!({
        ::miresidian::__require_instances_runtime();
        ::miresidian::InstancePolicy { mode: #mode, binding: #binding, execution: #execution }
    })
}

fn required<'a>(fields: &'a HashMap<String, Expr>, name: &str) -> Result<&'a Expr> {
    fields.get(name).ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!("instances 缺少 {name}"),
        )
    })
}

fn ident(value: &Expr) -> Result<String> {
    if let Expr::Path(path) = value
        && let Some(name) = path.path.get_ident()
    {
        return Ok(name.to_string());
    }
    Err(Error::new_spanned(value, "instances 配置值必须是标识符"))
}

fn execution_mode(value: &Expr) -> Result<proc_macro2::TokenStream> {
    match ident(value)?.as_str() {
        "sequential" => Ok(quote!(::miresidian::ExecutionMode::Sequential)),
        "parallel" => Ok(quote!(::miresidian::ExecutionMode::Parallel)),
        "broadcast" => Ok(quote!(::miresidian::ExecutionMode::Broadcast)),
        _ => Err(Error::new_spanned(
            value,
            "执行方式必须是 sequential、parallel 或 broadcast",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use syn::{Meta, Token, parse::Parser, punctuated::Punctuated};

    fn policy(source: &str) -> syn::Result<String> {
        Punctuated::<Meta, Token![,]>::parse_terminated
            .parse_str(source)
            .and_then(parse)
            .map(|tokens| tokens.to_string())
    }

    #[test]
    fn complete_mode_and_execution_matrix_matches_long_syntax() {
        for (short_mode, long_mode) in [
            ("single", "mode = single"),
            ("multiple, config", "mode = multiple, binding = config"),
            ("multiple, handle", "mode = multiple, binding = handle"),
        ] {
            for execution in ["sequential", "parallel", "broadcast"] {
                for selectable in [false, true] {
                    let (short_execution, long_execution) = if selectable {
                        (
                            format!("selectable({execution})"),
                            format!("execution = selectable, default_execution = {execution}"),
                        )
                    } else {
                        (execution.to_owned(), format!("execution = {execution}"))
                    };
                    assert_eq!(
                        policy(&format!("{short_mode}, {short_execution}")).unwrap(),
                        policy(&format!("{long_mode}, {long_execution}")).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn short_defaults_and_option_order_are_explicit() {
        let default = policy("mode = single, execution = sequential").unwrap();
        for short in ["", "single", "sequential", "single, sequential"] {
            assert_eq!(policy(short).unwrap(), default);
        }
        for binding in ["config", "handle"] {
            assert_eq!(
                policy(&format!("multiple, {binding}")).unwrap(),
                policy(&format!(
                    "mode = multiple, binding = {binding}, execution = sequential"
                ))
                .unwrap()
            );
        }
        assert_eq!(
            policy("multiple, handle, parallel").unwrap(),
            policy("parallel, handle, multiple").unwrap()
        );
        assert_eq!(
            policy("selectable(broadcast)").unwrap(),
            policy("mode = single, execution = selectable, default_execution = broadcast").unwrap()
        );
    }

    #[test]
    fn invalid_short_forms_and_incomplete_long_forms_stay_errors() {
        for source in [
            "multiple",
            "single, config",
            "config",
            "handle",
            "single, multiple",
            "single, single",
            "multiple, config, handle",
            "parallel, sequential",
            "parallel, selectable(sequential)",
            "selectable",
            "selectable()",
            "selectable(parallel, broadcast)",
            "selectable(unknown)",
            "multiple, binding = config",
            "mode = multiple, config, execution = sequential",
            "mode = single",
            "mode = multiple, binding = config",
            "mode = multiple, execution = sequential",
            "mode = single, execution = selectable",
        ] {
            assert!(policy(source).is_err(), "{source}");
        }
    }
}
