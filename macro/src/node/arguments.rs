//! 节点身份与 feature 分组入口；裸标记和分组归一到相同声明。

use super::lease::{self, LeaseDeclaration};
use syn::{
    Error, Ident, LitStr, Meta, Path, Result, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

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
            match keyword.to_string().as_str() {
                "instances" => {
                    if instances.is_some() {
                        return Err(Error::new_spanned(keyword, "instances 只能声明一次"));
                    }
                    instances = Some(super::instances::parse(
                        optional_group(input)?.unwrap_or_default(),
                    )?);
                }
                "lease" => {
                    if lease.is_some() {
                        return Err(Error::new_spanned(keyword, "lease 只能声明一次"));
                    }
                    lease = Some(match optional_group(input)? {
                        Some(values) => lease::parse(values)?,
                        None => lease::required(),
                    });
                }
                "platforms" => {
                    if platforms.is_some() {
                        return Err(Error::new_spanned(keyword, "platforms 只能声明一次"));
                    }
                    let content;
                    parenthesized!(content in input);
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
                        "node 子配置仅支持 instances、lease 及其分组和 platforms(...)",
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

fn optional_group(input: ParseStream<'_>) -> Result<Option<Punctuated<Meta, Token![,]>>> {
    if !input.peek(syn::token::Paren) {
        return Ok(None);
    }
    let content;
    parenthesized!(content in input);
    content.parse_terminated(Meta::parse, Token![,]).map(Some)
}

#[cfg(test)]
mod tests {
    use super::NodeArguments;

    #[test]
    fn bare_features_match_explicit_policies_and_omission_stays_disabled() {
        let bare: NodeArguments = syn::parse_quote!("节点", instances, lease);
        let long: NodeArguments = syn::parse_quote!(
            "节点",
            instances(mode = single, execution = sequential),
            lease(activation = required)
        );
        assert_eq!(
            bare.instances.unwrap().to_string(),
            long.instances.unwrap().to_string()
        );
        assert_eq!(
            bare.lease.unwrap().activation.to_string(),
            long.lease.unwrap().activation.to_string()
        );
        let omitted: NodeArguments = syn::parse_quote!("节点");
        assert!(omitted.instances.is_none());
        assert!(omitted.lease.is_none());
    }

    #[test]
    fn mixed_feature_styles_preserve_platforms() {
        let args: NodeArguments = syn::parse_quote!(
            "节点",
            lease(selectable(required)),
            platforms(crate::Adapter),
            instances(mode = multiple, binding = handle, execution = sequential)
        );
        assert_eq!(args.platforms.len(), 1);
        assert!(args.instances.is_some());
        assert!(args.lease.is_some());
    }

    #[test]
    fn duplicate_features_and_unscoped_options_are_rejected() {
        for source in [
            "\"节点\", instances, instances(single)",
            "\"节点\", instances(), instances",
            "\"节点\", lease, lease(required)",
            "\"节点\", lease(activation = required), lease",
            "\"节点\", execution = sequential",
            "\"节点\", instances = config",
            "\"节点\", lease = required",
        ] {
            assert!(syn::parse_str::<NodeArguments>(source).is_err(), "{source}");
        }
    }
}
