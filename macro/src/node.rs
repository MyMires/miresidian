//! `#[miresidian::node]` 结构体标注的展开逻辑。
//! #[miresidian::node] 结构体标注的展开逻辑。

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Error, Expr, ExprArray, ExprLit, Ident, ItemStruct, Lit, LitStr, Meta, Result, Token,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

struct LeaseDeclaration {
    resource: LitStr,
    configuration: Vec<LitStr>,
    activation: proc_macro2::TokenStream,
}

struct NodeArguments {
    name: LitStr,
    lease: Option<LeaseDeclaration>,
}

impl Parse for NodeArguments {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        if input.is_empty() {
            return Ok(Self { name, lease: None });
        }
        input.parse::<Token![,]>()?;
        let keyword: Ident = input.parse()?;
        if keyword != "lease" {
            return Err(Error::new_spanned(keyword, "node 子配置仅支持 lease(...)"));
        }
        let content;
        parenthesized!(content in input);
        let values = content.parse_terminated(Meta::parse, Token![,])?;
        if !input.is_empty() {
            return Err(input.error("node 只允许一个 lease(...) 子配置"));
        }
        Ok(Self {
            name,
            lease: Some(parse_lease_declaration(values)?),
        })
    }
}
/// 生成节点展示名称与租约生命周期桥接代码，供 node_impl 调度读取。
pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = parse_macro_input!(attribute as NodeArguments);
    let name = arguments.name;
    let item_struct = parse_macro_input!(item as ItemStruct);
    if item_struct.attrs.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "lease")
    }) {
        return Error::new_spanned(
            &item_struct,
            "lease 必须写为 #[node(\"名称\", lease(...))] 的子配置",
        )
        .into_compile_error()
        .into();
    }
    let lease = arguments.lease;
    let identifier = &item_struct.ident;

    let activation = lease
        .as_ref()
        .map(|lease| lease.activation.clone())
        .unwrap_or_else(|| quote!(::miresidian::LeaseActivation::Disabled));
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
            #[doc(hidden)]
            pub const __MIRESIDIAN_LEASE_ACTIVATION: ::miresidian::LeaseActivation = #activation;
        }

        #participant
    }
    .into()
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
