//! 初始化 Config 的实例身份与租约角色；两种角色独立解析。
use syn::{Attribute, Error, Expr, Lit, LitStr, Result, Token, punctuated::Punctuated};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Role {
    #[default]
    Default,
    Key,
    Ignore,
}

pub(super) struct Config {
    pub(super) definition: LitStr,
    pub(super) instance: Role,
    pub(super) lease: Role,
}

pub(super) fn annotation(attribute: &Attribute, initialization: bool) -> Result<Config> {
    let args = attribute.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
    let mut args = args.into_iter();
    let definition = match args.next() {
        Some(Expr::Lit(value)) => match value.lit {
            Lit::Str(value) => value,
            other => return Err(Error::new_spanned(other, "config 名称必须是字符串")),
        },
        _ => {
            return Err(Error::new_spanned(
                attribute,
                "config 必须先声明一个名称字符串",
            ));
        }
    };
    let mut config = Config {
        definition,
        instance: Role::Default,
        lease: Role::Default,
    };
    for arg in args {
        if !initialization {
            return Err(Error::new_spanned(
                arg,
                "instance/lease 角色只能用于 init_device 的 Config 参数",
            ));
        }
        let Expr::Call(call) = arg else {
            return Err(Error::new_spanned(
                arg,
                "config 只接受 instance(key/ignore) 和 lease(key/ignore)",
            ));
        };
        let Expr::Path(path) = call.func.as_ref() else {
            return Err(Error::new_spanned(&call, "config 角色名称无效"));
        };
        let role = if path.path.is_ident("instance") {
            &mut config.instance
        } else if path.path.is_ident("lease") {
            &mut config.lease
        } else {
            return Err(Error::new_spanned(
                &call,
                "config 只接受 instance 和 lease 角色",
            ));
        };
        if *role != Role::Default {
            return Err(Error::new_spanned(
                &call,
                "同一参数的 instance/lease 角色不能重复或冲突",
            ));
        }
        let value = match call.args.first() {
            Some(Expr::Path(value)) if call.args.len() == 1 => &value.path,
            _ => {
                return Err(Error::new_spanned(
                    &call,
                    "角色必须且只能指定 key 或 ignore",
                ));
            }
        };
        *role = if value.is_ident("key") {
            Role::Key
        } else if value.is_ident("ignore") {
            Role::Ignore
        } else {
            return Err(Error::new_spanned(value, "角色只支持 key 或 ignore"));
        };
    }
    Ok(config)
}

/// 有显式 key 时采用白名单，否则排除 ignore；顺序始终来自 Config 声明。
pub(super) fn instance_indices(parameters: &[super::definition::Parameter]) -> Vec<usize> {
    use super::definition::ParameterSource;
    let configs = parameters
        .iter()
        .filter(|p| p.source == ParameterSource::Config)
        .collect::<Vec<_>>();
    let explicit = configs.iter().any(|p| p.instance == Role::Key);
    configs
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            (if explicit {
                p.instance == Role::Key
            } else {
                p.instance != Role::Ignore
            })
            .then_some(i)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::annotation;
    #[test]
    fn rejects_unknown_duplicate_and_misplaced_roles() {
        for attribute in [
            syn::parse_quote!(#[config("x", instance(key), instance(ignore))]),
            syn::parse_quote!(#[config("x", lease(key), lease(key))]),
            syn::parse_quote!(#[config("x", lease(skip))]),
            syn::parse_quote!(#[config("x", instance())]),
            syn::parse_quote!(#[config("x", instance(key, ignore))]),
            syn::parse_quote!(#[config("x", other(key))]),
            syn::parse_quote!(#[config("x", "y")]),
        ] {
            assert!(annotation(&attribute, true).is_err());
        }
        assert!(annotation(&syn::parse_quote!(#[config("x", instance(key))]), false).is_err());
    }
}
