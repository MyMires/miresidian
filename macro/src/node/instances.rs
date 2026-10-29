//! instances 子配置的编译期约束与策略生成。

use quote::quote;
use std::collections::HashMap;
use syn::{Error, Expr, Meta, Result, Token, punctuated::Punctuated};

pub(super) fn parse(values: Punctuated<Meta, Token![,]>) -> Result<proc_macro2::TokenStream> {
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
    Ok(quote!({
        ::miresidian::__require_instances_runtime();
        ::miresidian::InstancePolicy { mode: #mode, binding: #binding, execution: #execution }
    }))
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
