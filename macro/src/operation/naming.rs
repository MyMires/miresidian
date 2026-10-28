//! Rust 标识符到测试平台名称的转换规则。

use syn::{Attribute, Expr, Lit, LitStr, Result, Token, punctuated::Punctuated};

/// 解析 `#[operation]` 的可选平台名称。
///
/// 形式仅为 `#[operation]` 或 `#[operation("名称")]`。共享调用策略是运行时级配置，
/// 因此拒绝第二个参数，避免产生未生效的声明。
pub(super) fn operation_name(attribute: &Attribute, method: &syn::Ident) -> Result<LitStr> {
    if matches!(attribute.meta, syn::Meta::Path(_)) {
        return Ok(platform_method_name(method));
    }
    let arguments = attribute.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
    if arguments.len() == 1
        && let Some(Expr::Lit(literal)) = arguments.first()
        && let Lit::Str(name) = &literal.lit
    {
        return Ok(name.clone());
    }
    Err(syn::Error::new_spanned(
        attribute,
        "#[operation] 只接受一个可选的字符串名称；共享执行方式由 InitDevice 配置决定",
    ))
}
/// 自动生成参数名称。
pub(super) fn parameter_name(argument: &syn::PatType) -> Result<LitStr> {
    Ok(platform_identifier_name(
        &rust_parameter_identifier(argument)?.ident,
    ))
}

/// 返回方法参数在 Rust 源码中的标识符，用于结构体级 lease 声明引用。
pub(super) fn rust_parameter_name(argument: &syn::PatType) -> Result<LitStr> {
    let identifier = rust_parameter_identifier(argument)?;
    Ok(LitStr::new(
        &identifier.ident.to_string(),
        identifier.ident.span(),
    ))
}

fn rust_parameter_identifier(argument: &syn::PatType) -> Result<&syn::PatIdent> {
    let syn::Pat::Ident(identifier) = argument.pat.as_ref() else {
        return Err(syn::Error::new_spanned(
            argument,
            "节点参数必须使用普通 Rust 标识符",
        ));
    };
    Ok(identifier)
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
