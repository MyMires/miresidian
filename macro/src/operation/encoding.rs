//! 显式 encoding 标注及其合成的格式选择输入。
use super::definition::{Parameter, ParameterSource};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, Lit, LitStr, Result, Token, punctuated::Punctuated};

#[derive(Clone)]
pub(super) struct Encoding {
    pub(super) formats: Vec<syn::Ident>,
    pub(super) selector: Option<LitStr>,
}
impl Encoding {
    pub(super) fn formats(&self) -> TokenStream {
        let formats = self.formats.iter().map(|encoding| {
            let variant = match encoding.to_string().as_str() {
                "ascii" => "Ascii",
                "gbk" => "Gbk",
                "utf8" => "Utf8",
                "utf16le" => "Utf16Le",
                "hex" => "Hex",
                "base64" => "Base64",
                _ => unreachable!(),
            };
            let variant = syn::Ident::new(variant, encoding.span());
            quote!(::miresidian::ByteTextFormat::#variant)
        });
        quote!(&[#(#formats),*])
    }
    pub(super) fn selection(&self, index: usize) -> TokenStream {
        let formats = self.formats();
        if let Some(name) = &self.selector {
            quote!(::miresidian::encoding::select_encoding(input, #index, #name, #formats)?)
        } else {
            quote!((#formats)[0])
        }
    }
    pub(super) fn parameter(&self) -> Option<Parameter> {
        self.selector.as_ref().map(|name| {
            let choices = self
                .formats
                .iter()
                .map(|encoding| match encoding.to_string().as_str() {
                    "ascii" => "ASCII",
                    "gbk" => "GBK",
                    "utf8" => "UTF8",
                    "utf16le" => "Unicode",
                    "hex" => "HEX",
                    "base64" => "Base64",
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>();
            Parameter {
                ty: syn::parse_quote!(String),
                name: name.clone(),
                rust_name: name.clone(),
                definition: LitStr::new(
                    &format!("{}:{}", name.value(), choices.join("/")),
                    name.span(),
                ),
                default: Some(LitStr::new(choices[0], name.span())),
                source: ParameterSource::Input,
                batch: false,
                encoding: None,
                instance: super::config::Role::Default,
                lease: super::config::Role::Default,
            }
        })
    }
}

/// 保留原名称列表语法，同时解析 input/output 的增强选项。
pub(super) fn annotation(
    attribute: &syn::Attribute,
    allow_batch: bool,
) -> Result<(Vec<LitStr>, bool, Option<Encoding>)> {
    let args = attribute.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
    let mut names = Vec::new();
    let mut batch = false;
    let mut encoding = None;
    for arg in args {
        match arg {
            Expr::Lit(value) if !batch && encoding.is_none() => {
                let Lit::Str(value) = value.lit else {
                    return Err(Error::new_spanned(value, "名称必须是字符串"));
                };
                names.push(value);
            }
            Expr::Path(path) if allow_batch && path.path.is_ident("batch") && !batch => {
                batch = true
            }
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path) if path.path.is_ident("encoding"))
                    && encoding.is_none() =>
            {
                encoding = Some(parse_encoding(&call)?);
            }
            other => return Err(Error::new_spanned(other, "无效或重复的参数标注选项")),
        }
    }
    if names.is_empty() || ((batch || encoding.is_some()) && names.len() != 1) {
        return Err(Error::new_spanned(
            attribute,
            "增强标注必须且只能声明一个名称",
        ));
    }
    if let Some(encoding) = &mut encoding
        && encoding.formats.len() > 1
        && encoding.selector.is_none()
    {
        let definition = names[0].value();
        let name = definition
            .split_once(':')
            .map_or(definition.as_str(), |(name, _)| name)
            .trim();
        let selector = LitStr::new(&format!("{name}格式"), names[0].span());
        if name.is_empty() || validate_selector(&selector).is_err() {
            return Err(Error::new_spanned(
                &names[0],
                "无法自动生成有效格式选择名称，请在 encoding 首项显式指定名称",
            ));
        }
        encoding.selector = Some(selector);
    }
    Ok((names, batch, encoding))
}

/// 新旧语法最终使用同一个 Encoding，运行时转换与槽位生成不分支。
fn parse_encoding(call: &syn::ExprCall) -> Result<Encoding> {
    if matches!(call.args.first(), Some(Expr::Path(_) | Expr::Lit(_))) {
        parse_short_encoding(call)
    } else {
        parse_long_encoding(call)
    }
}

fn parse_short_encoding(call: &syn::ExprCall) -> Result<Encoding> {
    let mut formats = Vec::new();
    let mut selector = None;
    let mut options = call.args.iter();
    let named = matches!(options.clone().next(), Some(Expr::Lit(_)));
    if named {
        let Some(Expr::Lit(value)) = options.next() else {
            unreachable!()
        };
        let Lit::Str(name) = &value.lit else {
            return Err(Error::new_spanned(value, "encoding 首项名称必须是字符串"));
        };
        validate_selector(name)?;
        selector = Some(name.clone());
    }
    let mut ended = false;
    for option in options {
        match option {
            Expr::Path(path) if !ended => {
                let ident = path
                    .path
                    .get_ident()
                    .ok_or_else(|| Error::new_spanned(path, "格式名称无效"))?;
                if !matches!(
                    ident.to_string().as_str(),
                    "ascii" | "gbk" | "utf8" | "utf16le" | "hex" | "base64"
                ) || formats.contains(ident)
                {
                    return Err(Error::new_spanned(ident, "未知或重复的 encoding 格式"));
                }
                formats.push(ident.clone());
            }
            Expr::Assign(assign)
                if matches!(assign.left.as_ref(), Expr::Path(key) if key.path.is_ident("selector"))
                    && selector.is_none() =>
            {
                let Expr::Lit(value) = assign.right.as_ref() else {
                    return Err(Error::new_spanned(option, "selector 必须是字符串"));
                };
                let Lit::Str(value) = &value.lit else {
                    return Err(Error::new_spanned(option, "selector 必须是字符串"));
                };
                validate_selector(value)?;
                selector = Some(value.clone());
                ended = true;
            }
            _ => {
                return Err(Error::new_spanned(
                    option,
                    "encoding 短写仅支持首项名称、格式列表和末尾 selector，禁止重复命名或混用 formats 数组",
                ));
            }
        }
    }
    if formats.is_empty() {
        return Err(Error::new_spanned(call, "encoding 必须声明至少一种格式"));
    }
    if formats.len() == 1 && selector.is_some() {
        return Err(Error::new_spanned(call, "单格式不接受选择器名称"));
    }
    Ok(Encoding { formats, selector })
}

fn parse_long_encoding(call: &syn::ExprCall) -> Result<Encoding> {
    let mut formats = None;
    let mut selector = None;
    for option in &call.args {
        let Expr::Assign(assign) = option else {
            return Err(Error::new_spanned(
                option,
                "encoding 选项必须为 key = value",
            ));
        };
        let Expr::Path(key) = assign.left.as_ref() else {
            return Err(Error::new_spanned(option, "encoding 选项名称无效"));
        };
        if key.path.is_ident("formats") && formats.is_none() {
            let Expr::Array(array) = assign.right.as_ref() else {
                return Err(Error::new_spanned(option, "formats 必须是格式数组"));
            };
            let mut list = Vec::new();
            for value in &array.elems {
                let Expr::Path(path) = value else {
                    return Err(Error::new_spanned(value, "格式必须是标识符"));
                };
                let Some(ident) = path.path.get_ident() else {
                    return Err(Error::new_spanned(value, "格式名称无效"));
                };
                if !matches!(
                    ident.to_string().as_str(),
                    "ascii" | "gbk" | "utf8" | "utf16le" | "hex" | "base64"
                ) || list.contains(ident)
                {
                    return Err(Error::new_spanned(value, "未知或重复的 encoding 格式"));
                }
                list.push(ident.clone());
            }
            if list.is_empty() {
                return Err(Error::new_spanned(array, "formats 不能为空"));
            }
            formats = Some(list);
        } else if key.path.is_ident("selector") && selector.is_none() {
            let Expr::Lit(value) = assign.right.as_ref() else {
                return Err(Error::new_spanned(option, "selector 必须是字符串"));
            };
            let Lit::Str(value) = &value.lit else {
                return Err(Error::new_spanned(option, "selector 必须是字符串"));
            };
            validate_selector(value)?;
            selector = Some(value.clone());
        } else {
            return Err(Error::new_spanned(
                option,
                "未知或重复的 encoding 选项；只支持 formats 和 selector",
            ));
        }
    }
    let formats = formats.ok_or_else(|| Error::new_spanned(call, "encoding 必须声明 formats"))?;
    if formats.len() == 1 && selector.is_some() {
        return Err(Error::new_spanned(call, "单格式不接受选择器名称"));
    }
    Ok(Encoding { formats, selector })
}

fn validate_selector(name: &LitStr) -> Result<()> {
    if name.value().trim().is_empty() || name.value().contains([':', '/', '|']) {
        return Err(Error::new_spanned(name, "selector 必须是非空参数名称"));
    }
    Ok(())
}

/// 平台输入包括参数就地附加的格式选择，以及末尾的输出格式选择。
pub(super) fn platform_parameters(
    parameters: &[Parameter],
    codecs: &[Option<Encoding>],
) -> Vec<Parameter> {
    let mut result = Vec::new();
    for parameter in parameters {
        result.push(parameter.clone());
        if let Some(selector) = parameter.encoding.as_ref().and_then(Encoding::parameter) {
            result.push(selector);
        }
    }
    result.extend(codecs.iter().flatten().filter_map(Encoding::parameter));
    result
}

#[cfg(test)]
mod tests {
    use super::annotation;
    #[test]
    fn rejects_invalid_format_contracts() {
        for attr in [
            syn::parse_quote!(#[input("数据", codec(formats = [hex]))]),
            syn::parse_quote!(#[input("数据", encoding(formats = []))]),
            syn::parse_quote!(#[input("数据", encoding(formats = [hex, hex], selector = "格式"))]),
            syn::parse_quote!(#[input("数据", encoding(formats = [hex], selector = "格式"))]),
            syn::parse_quote!(#[input("数据", encoding(formats = [hex], default = hex))]),
            syn::parse_quote!(#[input("数据", encoding(formats = [json]))]),
        ] {
            assert!(annotation(&attr, true).is_err());
        }
    }

    #[test]
    fn all_fixed_formats_share_the_same_contract_as_long_syntax() {
        for name in ["ascii", "gbk", "utf8", "utf16le", "hex", "base64"] {
            for target in ["input", "output"] {
                let short = syn::parse::Parser::parse_str(
                    syn::Attribute::parse_outer,
                    &format!("#[{target}(\"数据\", encoding({name}))]"),
                )
                .unwrap()
                .remove(0);
                let long = syn::parse::Parser::parse_str(
                    syn::Attribute::parse_outer,
                    &format!("#[{target}(\"数据\", encoding(formats = [{name}]))]"),
                )
                .unwrap()
                .remove(0);
                let (_, _, short) = annotation(&short, target == "input").unwrap();
                let (_, _, long) = annotation(&long, target == "input").unwrap();
                let (short, long) = (short.unwrap(), long.unwrap());
                assert_eq!(short.formats().to_string(), long.formats().to_string());
                assert!(short.parameter().is_none());
                assert!(long.parameter().is_none());
            }
        }
    }

    #[test]
    fn selectors_and_batch_order_preserve_metadata() {
        let short: syn::Attribute = syn::parse_quote!(#[input("数据", encoding(utf8, hex, base64, selector = "格式"), batch)]);
        let long: syn::Attribute = syn::parse_quote!(#[input("数据", batch, encoding(selector = "格式", formats = [utf8, hex, base64]))]);
        let (_, short_batch, short) = annotation(&short, true).unwrap();
        let (_, long_batch, long) = annotation(&long, true).unwrap();
        assert!(short_batch && long_batch);
        let (short, long) = (short.unwrap(), long.unwrap());
        assert_eq!(short.formats().to_string(), long.formats().to_string());
        let (short, long) = (short.parameter().unwrap(), long.parameter().unwrap());
        assert_eq!(short.definition.value(), "格式:UTF8/HEX/Base64");
        assert_eq!(short.definition.value(), long.definition.value());
        assert_eq!(
            short.default.unwrap().value(),
            long.default.unwrap().value()
        );
    }

    #[test]
    fn automatic_and_named_selectors_normalize_to_the_same_contract() {
        for target in ["input", "output"] {
            let mut expected = None;
            for options in [
                "utf8, hex, base64, ascii, gbk, utf16le",
                "\"数据格式\", utf8, hex, base64, ascii, gbk, utf16le,",
                "utf8, hex, base64, ascii, gbk, utf16le, selector = \"数据格式\"",
                "formats = [utf8, hex, base64, ascii, gbk, utf16le]",
                "selector = \"数据格式\", formats = [utf8, hex, base64, ascii, gbk, utf16le]",
            ] {
                let attr = syn::parse::Parser::parse_str(
                    syn::Attribute::parse_outer,
                    &format!("#[{target}(\" 数据 :描述\", encoding({options}))]"),
                )
                .unwrap()
                .remove(0);
                let encoding = annotation(&attr, target == "input").unwrap().2.unwrap();
                let parameter = encoding.parameter().unwrap();
                let actual = (
                    encoding.formats().to_string(),
                    parameter.name.value(),
                    parameter.definition.value(),
                    parameter.default.unwrap().value(),
                );
                assert_eq!(actual.1, "数据格式");
                assert_eq!(actual.3, "UTF8");
                if let Some(expected) = &expected {
                    assert_eq!(&actual, expected);
                } else {
                    expected = Some(actual);
                }
            }
        }
    }

    #[test]
    fn short_format_errors_do_not_gain_implicit_defaults_or_aliases() {
        let attributes: Vec<syn::Attribute> = vec![
            syn::parse_quote!(#[input("数据", format(hex))]),
            syn::parse_quote!(#[input("数据", encoding("名称"))]),
            syn::parse_quote!(#[input("数据", encoding("名称", hex))]),
            syn::parse_quote!(#[input("数据", encoding(" ", utf8, hex))]),
            syn::parse_quote!(#[input("数据", encoding("非法:名称", utf8, hex))]),
            syn::parse_quote!(#[input("数据", encoding("非法/名称", utf8, hex))]),
            syn::parse_quote!(#[input("数据", encoding("非法|名称", utf8, hex))]),
            syn::parse_quote!(#[input("数据", encoding(1, utf8, hex))]),
            syn::parse_quote!(#[input("数据", encoding("名称", "其他", utf8, hex))]),
            syn::parse_quote!(#[input("数据", encoding(utf8, hex, "名称"))]),
            syn::parse_quote!(#[input("数据", encoding("名称", utf8, hex, selector = "名称"))]),
            syn::parse_quote!(#[input("数据", encoding("名称", formats = [utf8, hex]))]),
            syn::parse_quote!(#[input("数据", encoding(uft8, hex))]),
            syn::parse_quote!(#[input(" :描述", encoding(utf8, hex))]),
            syn::parse_quote!(#[input("非法/名称", encoding(utf8, hex))]),
            syn::parse_quote!(#[input("数据", encoding())]),
            syn::parse_quote!(#[input("数据", encoding(hex, hex, selector = "格式"))]),
            syn::parse_quote!(#[input("数据", encoding(json))]),
            syn::parse_quote!(#[input("数据", encoding(hex, selector = "格式"))]),
            syn::parse_quote!(#[input("数据", encoding(hex, utf8, selector = ""))]),
            syn::parse_quote!(#[input("数据", encoding(hex, utf8, selector = "非法/格式"))]),
            syn::parse_quote!(#[input("数据", encoding(hex, utf8, selector = 1))]),
            syn::parse_quote!(#[input("数据", encoding(hex, utf8, selector = "格式", selector = "格式2"))]),
            syn::parse_quote!(#[input("数据", encoding(hex, formats = [hex]))]),
            syn::parse_quote!(#[input("数据", encoding(formats = [hex], utf8))]),
            syn::parse_quote!(#[input("数据", encoding(hex, utf8, selector = "格式", base64))]),
            syn::parse_quote!(#[input("数据", encoding(hex), encoding(formats = [hex]))]),
            syn::parse_quote!(#[input("数据", encoding = hex)]),
        ];
        for attribute in attributes {
            assert!(annotation(&attribute, true).is_err());
        }
    }
}
