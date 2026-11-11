//! 从标准无字段枚举生成通用 codec 实现，不拼接任何平台候选语法。

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, LitStr, Result, parse_macro_input};

pub(crate) fn expand(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    generate(&item)
        .and_then(crate::framework::rewrite_paths)
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

fn generate(item: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &item.generics,
            "NodeEnum 不支持泛型枚举",
        ));
    }
    let Data::Enum(data) = &item.data else {
        return Err(Error::new_spanned(item, "NodeEnum 只能用于枚举"));
    };
    if data.variants.is_empty() {
        return Err(Error::new_spanned(item, "NodeEnum 需要至少一个无字段成员"));
    }
    let mut options = Vec::new();
    let mut parse = Vec::new();
    let mut names = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new_spanned(variant, "NodeEnum 只支持无字段成员"));
        }
        let ident = &variant.ident;
        let actual = ident.to_string();
        let actual = LitStr::new(actual.strip_prefix("r#").unwrap_or(&actual), ident.span());
        let cfg = crate::framework::conditional_attributes(&variant.attrs);
        options
            .push(quote!(#(#cfg)* ::miresidian::EnumOption { value: #actual, display: #actual }));
        parse.push(quote!(#(#cfg)* #actual => Ok(Self::#ident),));
        names.push(quote!(#(#cfg)* Self::#ident => #actual,));
    }
    let ident = &item.ident;
    let cfg = crate::framework::conditional_attributes(&item.attrs);
    Ok(quote! {
        #(#cfg)*
        impl ::miresidian::NodeInput for #ident {
            const OPTIONS: &'static [::miresidian::EnumOption] = &[#(#options),*];

            fn parse_text(value: &str, field: &str) -> ::miresidian::MiresidianResult<Self> {
                match value.trim() {
                    #(#parse)*
                    _ => Err(::miresidian::MiresidianError::InvalidArgument {
                        field: field.to_owned(),
                        message: ::std::format!("未知枚举值：{value}"),
                    }),
                }
            }

            fn normalize_text(value: &str, field: &str) -> ::miresidian::MiresidianResult<::std::string::String> {
                let value = <Self as ::miresidian::NodeInput>::parse_text(value, field)?;
                Ok(match value { #(#names)* }.to_owned())
            }
        }

        #(#cfg)*
        impl ::miresidian::NodeOutput for #ident {
            fn into_output(self) -> ::miresidian::MiresidianResult<::miresidian::InvocationOutput> {
                let value = match self { #(#names)* };
                Ok(::miresidian::InvocationOutput::new(::std::vec![value.to_owned()]))
            }
        }

        #(#cfg)*
        impl ::miresidian::NodeCollectionElement for #ident {
            const OPTIONS: &'static [::miresidian::EnumOption] = <Self as ::miresidian::NodeInput>::OPTIONS;

            fn parse_collection(value: &str, field: &str) -> ::miresidian::MiresidianResult<::std::vec::Vec<Self>> {
                <::std::vec::Vec<::std::string::String> as ::miresidian::NodeInput>::parse_text(value, field)?
                    .iter().map(|item| <Self as ::miresidian::NodeInput>::parse_text(item, field)).collect()
            }

            fn format_collection(values: &[Self]) -> ::miresidian::MiresidianResult<::std::string::String> {
                let values = values.iter().map(|value| {
                    match value { #(#names)* }.to_owned()
                }).collect::<::std::vec::Vec<_>>();
                <::std::string::String as ::miresidian::NodeCollectionElement>::format_collection(&values)
            }
        }
    })
}
