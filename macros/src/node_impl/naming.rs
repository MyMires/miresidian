//! Rust 标识符到测试平台名称的转换规则。

use super::definition::ExecutionMode;
use syn::{Attribute, Ident, LitStr, Result, Token, parse::Parse, punctuated::Punctuated};

/// `#[operation]` 属性的完整参数：名称与可选执行方式。
pub(super) struct OperationAttribute {
    values: Punctuated<OperationArgument, Token![,]>,
}

/// 单个 `#[operation]` 参数；名称是字符串，执行方式是裸标识符。
enum OperationArgument {
    Name(LitStr),
    Execution(Ident),
}

impl Parse for OperationAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            values: input.parse_terminated(OperationArgument::parse, Token![,])?,
        })
    }
}

impl OperationArgument {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            return Ok(Self::Name(input.parse()?));
        }
        Ok(Self::Execution(input.parse()?))
    }
}

/// 解析 `#[operation]` 的名称与可选执行方式。
///
/// 形式为 `#[operation]`、`#[operation("名称")]` 或 `#[operation("名称", parallel|sequential)]`；
/// 未声明执行方式时默认顺序执行。
pub(super) fn operation_name(
    attribute: &Attribute,
    method: &syn::Ident,
) -> Result<(LitStr, ExecutionMode)> {
    if matches!(attribute.meta, syn::Meta::Path(_)) {
        return Ok((platform_method_name(method), ExecutionMode::Sequential));
    }
    let arguments: OperationAttribute = attribute.parse_args()?;
    let mut name = None;
    let mut execution = None;
    for argument in arguments.values {
        match argument {
            OperationArgument::Name(value) => {
                if name.replace(value).is_some() {
                    return Err(syn::Error::new_spanned(
                        attribute,
                        "#[operation] 只能声明一个名称",
                    ));
                }
            }
            OperationArgument::Execution(value) => {
                if execution.replace(parse_execution(&value)?).is_some() {
                    return Err(syn::Error::new_spanned(
                        attribute,
                        "#[operation] 只能声明一个执行方式",
                    ));
                }
            }
        }
    }
    let name = name
        .ok_or_else(|| syn::Error::new_spanned(attribute, "#[operation] 必须声明一个字符串名称"))?;
    Ok((name, execution.unwrap_or(ExecutionMode::Sequential)))
}

/// 解析执行方式关键字；仅接受 `parallel` 与 `sequential`。
fn parse_execution(value: &Ident) -> Result<ExecutionMode> {
    match value.to_string().as_str() {
        "parallel" => Ok(ExecutionMode::Parallel),
        "sequential" => Ok(ExecutionMode::Sequential),
        other => Err(syn::Error::new_spanned(
            value,
            format!("未知执行方式 {other:?}；只支持 parallel 或 sequential"),
        )),
    }
}
/// 自动生成参数名称。
pub(super) fn parameter_name(argument: &syn::PatType) -> Result<LitStr> {
    let syn::Pat::Ident(identifier) = argument.pat.as_ref() else {
        return Err(syn::Error::new_spanned(
            argument,
            "自动生成参数名称时，参数必须使用普通标识符",
        ));
    };
    Ok(platform_identifier_name(&identifier.ident))
}
pub(super) fn platform_method_name(method: &syn::Ident) -> LitStr {
    platform_identifier_name(method)
}
/// 将普通 Rust 标识符转换为平台使用的 PascalCase。
/// 固定入口的特殊拼写（例如 `GetPortNO`）由运行时契约处理，`port_no` 因而得到 `PortNo`。
pub(super) fn platform_identifier_name(identifier: &syn::Ident) -> LitStr {
    let converted = identifier
        .to_string()
        .split('_')
        .filter(|part| !part.is_empty())
        .map(pascal_word)
        .collect::<String>();
    LitStr::new(&converted, identifier.span())
}
fn pascal_word(word: &str) -> String {
    let mut characters = word.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{platform_identifier_name, platform_method_name};
    use quote::format_ident;
    #[test]
    fn 自动方法名转换为平台形式() {
        let init = format_ident!("init_device");
        let parameter = format_ident!("port_no");
        let read = format_ident!("read_text");
        assert_eq!(platform_method_name(&init).value(), "InitDevice");
        assert_eq!(platform_method_name(&read).value(), "ReadText");
        assert_eq!(platform_identifier_name(&parameter).value(), "PortNo");
    }
}
