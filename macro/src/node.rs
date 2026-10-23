//! `#[miresidian::node]` 结构体标注的展开逻辑。
//! #[miresidian::node] 结构体标注的展开逻辑。

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprArray, ExprLit, ItemStruct, Lit, LitStr, Meta, Result, Token,
    parse_macro_input, punctuated::Punctuated,
};

struct LeaseDeclaration {
    resource: LitStr,
    configuration: Vec<LitStr>,
}

/// 生成节点展示名称与租约生命周期桥接代码，供 node_impl 调度读取。
pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let name = parse_macro_input!(attribute as LitStr);
    let mut item_struct = parse_macro_input!(item as ItemStruct);
    let lease = match take_lease_attribute(&mut item_struct.attrs) {
        Ok(lease) => lease,
        Err(error) => return error.into_compile_error().into(),
    };
    let identifier = &item_struct.ident;

    let (acquire, release, participant) = match lease {
        Some(lease) => {
            let resource = lease.resource;
            let configuration = lease.configuration;
            (
                quote! {
                    #[doc(hidden)]
                    fn __miresidian_lease_acquire(&mut self, config: &[::std::string::String], _input: &[::std::string::String]) -> ::miresidian::MiresidianResult<()> {
                        let values = Self::__miresidian_lease_parameters(config)?;
                        let parameters = values.into_iter().map(|(name, value)| ::miresidian::lease::LeaseParameter::new(name, value)).collect();
                        let request = ::miresidian::lease::LeaseRequest::from_parameters(parameters, #resource, &[#(#configuration),*]).map_err(|error| ::miresidian::MiresidianError::Failure(error.to_string()))?;
                        ::miresidian::lease::LeaseParticipant::acquire_lease(self, request)
                    }
                },
                quote! {
                    #[doc(hidden)]
                    fn __miresidian_lease_release(&mut self) -> ::miresidian::MiresidianResult<()> {
                        ::miresidian::lease::LeaseParticipant::release_lease(self)
                    }
                },
                quote! {
                    impl ::miresidian::lease::LeaseParticipant for #identifier {
                        fn acquire_lease(&mut self, request: ::miresidian::lease::LeaseRequest) -> ::miresidian::MiresidianResult<()> {
                            <#identifier>::lease_acquire(self, request)
                        }

                        fn release_lease(&mut self) -> ::miresidian::MiresidianResult<()> {
                            <#identifier>::lease_release(self)
                        }
                    }
                },
            )
        }
        None => (
            quote! {
                #[doc(hidden)]
                fn __miresidian_lease_acquire(&mut self, _config: &[::std::string::String], _input: &[::std::string::String]) -> ::miresidian::MiresidianResult<()> {
                    Ok(())
                }
            },
            quote! {
                #[doc(hidden)]
                fn __miresidian_lease_release(&mut self) -> ::miresidian::MiresidianResult<()> {
                    Ok(())
                }
            },
            quote! {},
        ),
    };

    quote! {
        #item_struct

        impl #identifier {
            #[doc(hidden)]
            pub const __MIRESIDIAN_NODE_NAME: &'static str = #name;
            #acquire
            #release
        }

        #participant
    }
    .into()
}

fn take_lease_attribute(attributes: &mut Vec<Attribute>) -> Result<Option<LeaseDeclaration>> {
    let Some(index) = attributes.iter().position(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "lease")
    }) else {
        return Ok(None);
    };
    let attribute = attributes.remove(index);
    let values = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut resource = None;
    let mut configuration = None;
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
                let Expr::Array(ExprArray { elems, .. }) = value.value else {
                    return Err(Error::new_spanned(value, "configuration 必须是字符串数组"));
                };
                let values = elems
                    .iter()
                    .map(|value| string_literal(value, "configuration"))
                    .collect::<Result<Vec<_>>>()?;
                if values.is_empty() {
                    return Err(Error::new_spanned(
                        attribute,
                        "configuration 至少需要一个字段",
                    ));
                }
                configuration = Some(values);
            }
            _ => {
                return Err(Error::new_spanned(
                    value,
                    "lease 只接受 resource 和 configuration 参数",
                ));
            }
        }
    }
    Ok(Some(LeaseDeclaration {
        resource: resource.ok_or_else(|| Error::new_spanned(&attribute, "lease 缺少 resource"))?,
        configuration: configuration
            .ok_or_else(|| Error::new_spanned(&attribute, "lease 缺少 configuration"))?,
    }))
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
