//! 显式 codec 标注及其合成的格式选择输入。
use super::definition::{Parameter, ParameterSource};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, Lit, LitStr, Result, Token, punctuated::Punctuated};

#[derive(Clone)]
pub(super) struct Codec {
    pub(super) formats: Vec<syn::Ident>,
    pub(super) selector: Option<LitStr>,
}
impl Codec {
    pub(super) fn formats(&self) -> TokenStream {
        let formats = self.formats.iter().map(|format| {
            let variant = match format.to_string().as_str() {
                "ascii" => "Ascii",
                "gbk" => "Gbk",
                "utf8" => "Utf8",
                "utf16le" => "Utf16Le",
                "hex" => "Hex",
                "base64" => "Base64",
                _ => unreachable!(),
            };
            let variant = syn::Ident::new(variant, format.span());
            quote!(::miresidian::ByteTextFormat::#variant)
        });
        quote!(&[#(#formats),*])
    }
    pub(super) fn selection(&self, index: usize) -> TokenStream {
        let formats = self.formats();
        if let Some(name) = &self.selector {
            quote!(::miresidian::codec::select_format(input, #index, #name, #formats)?)
        } else {
            quote!((#formats)[0])
        }
    }
    pub(super) fn parameter(&self) -> Option<Parameter> {
        self.selector.as_ref().map(|name| {
            let choices = self
                .formats
                .iter()
                .map(|format| match format.to_string().as_str() {
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
                codec: None,
            }
        })
    }
}

/// 保留原名称列表语法，同时解析 input/output 的增强选项。
pub(super) fn annotation(
    attribute: &syn::Attribute,
    allow_batch: bool,
) -> Result<(Vec<LitStr>, bool, Option<Codec>)> {
    let args = attribute.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
    let mut names = Vec::new();
    let mut batch = false;
    let mut codec = None;
    for arg in args {
        match arg {
            Expr::Lit(value) if !batch && codec.is_none() => {
                let Lit::Str(value) = value.lit else {
                    return Err(Error::new_spanned(value, "名称必须是字符串"));
                };
                names.push(value);
            }
            Expr::Path(path) if allow_batch && path.path.is_ident("batch") && !batch => {
                batch = true
            }
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path) if path.path.is_ident("codec"))
                    && codec.is_none() =>
            {
                let mut formats = None;
                let mut selector = None;
                for option in &call.args {
                    let Expr::Assign(assign) = option else {
                        return Err(Error::new_spanned(option, "codec 选项必须为 key = value"));
                    };
                    let Expr::Path(key) = assign.left.as_ref() else {
                        return Err(Error::new_spanned(option, "codec 选项名称无效"));
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
                                return Err(Error::new_spanned(value, "未知或重复的 codec 格式"));
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
                        if value.value().trim().is_empty()
                            || value.value().contains([':', '/', '|'])
                        {
                            return Err(Error::new_spanned(value, "selector 必须是非空参数名称"));
                        }
                        selector = Some(value.clone());
                    } else {
                        return Err(Error::new_spanned(
                            option,
                            "未知或重复的 codec 选项；只支持 formats 和 selector",
                        ));
                    }
                }
                let formats =
                    formats.ok_or_else(|| Error::new_spanned(&call, "codec 必须声明 formats"))?;
                if (formats.len() > 1) != selector.is_some() {
                    return Err(Error::new_spanned(
                        &call,
                        "单格式不接受 selector，多格式必须声明 selector",
                    ));
                }
                codec = Some(Codec { formats, selector });
            }
            other => return Err(Error::new_spanned(other, "无效或重复的参数标注选项")),
        }
    }
    if names.is_empty() || ((batch || codec.is_some()) && names.len() != 1) {
        return Err(Error::new_spanned(
            attribute,
            "增强标注必须且只能声明一个名称",
        ));
    }
    Ok((names, batch, codec))
}

/// 平台输入包括参数就地附加的格式选择，以及末尾的输出格式选择。
pub(super) fn platform_parameters(
    parameters: &[Parameter],
    codecs: &[Option<Codec>],
) -> Vec<Parameter> {
    let mut result = Vec::new();
    for parameter in parameters {
        result.push(parameter.clone());
        if let Some(selector) = parameter.codec.as_ref().and_then(Codec::parameter) {
            result.push(selector);
        }
    }
    result.extend(codecs.iter().flatten().filter_map(Codec::parameter));
    result
}

#[cfg(test)]
mod tests {
    use super::annotation;
    #[test]
    fn rejects_invalid_codec_contracts() {
        for attr in [
            syn::parse_quote!(#[input("数据", codec(formats = []))]),
            syn::parse_quote!(#[input("数据", codec(formats = [hex, hex], selector = "格式"))]),
            syn::parse_quote!(#[input("数据", codec(formats = [hex], selector = "格式"))]),
            syn::parse_quote!(#[input("数据", codec(formats = [hex, utf8]))]),
            syn::parse_quote!(#[input("数据", codec(formats = [hex], default = hex))]),
            syn::parse_quote!(#[input("数据", codec(formats = [json]))]),
        ] {
            assert!(annotation(&attr, true).is_err());
        }
    }
}
