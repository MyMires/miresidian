//! 从标准无字段枚举生成通用 codec 实现，不拼接任何平台候选语法。

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, Ident, LitStr, Result, Token, parse_macro_input,
};

pub(crate) fn expand(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    generate(&item)
        .and_then(crate::framework::rewrite_paths)
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

fn generate(item: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    if let Some(attribute) = item
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("node_enum"))
    {
        return Err(Error::new_spanned(
            attribute,
            "node_enum 显示值只能标注在枚举成员上",
        ));
    }
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
        let display = display_value(&variant.attrs)?.unwrap_or_else(|| actual.clone());
        let cfg = crate::framework::conditional_attributes(&variant.attrs);
        options
            .push(quote!(#(#cfg)* ::miresidian::EnumOption { value: #actual, display: #display }));
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

/// 只读取平台无关显示文字；保留原文供各适配器执行自身语法和歧义校验。
fn display_value(attributes: &[Attribute]) -> Result<Option<LitStr>> {
    let mut display = None;
    for attribute in attributes
        .iter()
        .filter(|attr| attr.path().is_ident("node_enum"))
    {
        if display.is_some() {
            return Err(Error::new_spanned(
                attribute,
                "每个枚举成员只能标注一个 node_enum 显示值",
            ));
        }
        let value = attribute.parse_args_with(|input: syn::parse::ParseStream<'_>| {
            let value: LitStr = if input.peek(LitStr) {
                input.parse()?
            } else {
                let key: Ident = input.parse()?;
                if key != "display" {
                    return Err(Error::new_spanned(
                        key,
                        "node_enum 仅支持 display = \"显示值\" 或 \"显示值\" 简写",
                    ));
                }
                input.parse::<Token![=]>()?;
                input.parse()?
            };
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            if !input.is_empty() {
                return Err(input.error("node_enum 只接受一个显示值，不可重复或混用长短写"));
            }
            if value.value().trim().is_empty() {
                return Err(Error::new_spanned(
                    value,
                    "node_enum 显示值不能为空或全为空白",
                ));
            }
            Ok(value)
        })?;
        display = Some(value);
    }
    Ok(display)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse::Parser;

    #[test]
    fn display_forms_accept_trailing_comma_and_preserve_platform_text() {
        for text in [
            "#[node_enum(\" A/B \")]",
            "#[node_enum(\" A/B \",)]",
            "#[node_enum(display = \" A/B \")]",
            "#[node_enum(display = \" A/B \",)]",
        ] {
            let attributes = Attribute::parse_outer.parse_str(text).unwrap();
            assert_eq!(
                display_value(&attributes).unwrap().unwrap().value(),
                " A/B "
            );
        }
    }
}
