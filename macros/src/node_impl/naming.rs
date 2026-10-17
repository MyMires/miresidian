//! Rust 标识符到测试平台名称的转换规则。

use super::definition::names;
use syn::{Attribute, LitStr, Result};

/// 未指定名称时，使用方法名的 PascalCase 形式。
pub(super) fn operation_name(attribute: &Attribute, method: &syn::Ident) -> Result<LitStr> {
    if matches!(attribute.meta, syn::Meta::Path(_)) {
        return Ok(platform_method_name(method));
    }
    let mut values = names(attribute, "operation")?;
    if values.len() != 1 {
        return Err(syn::Error::new_spanned(
            attribute,
            "#[operation] 必须且只能声明一个名称",
        ));
    }
    Ok(values.remove(0))
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
