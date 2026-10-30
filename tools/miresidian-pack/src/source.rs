//! 静态读取 Rust 节点属性；不展开宏或条件编译。
use std::{fs, path::Path};
use syn::{
    Attribute, Expr, Item, LitStr, Token,
    parse::{Parse, ParseStream},
};

/// 从节点源码中读取节点属性宏的显示名；不含节点属性的 crate 返回 `None`。
pub(crate) fn find_node_name(source_directory: &Path) -> Result<Option<String>, String> {
    let mut pending = vec![source_directory.to_owned()];
    let mut names = Vec::new();

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("无法读取节点源码目录 {}：{error}", directory.display()))?
        {
            let entry = entry.map_err(|error| format!("无法读取节点源码条目：{error}"))?;
            let path = entry.path();
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }

            let source = fs::read_to_string(&path)
                .map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
            let syntax = syn::parse_file(&source)
                .map_err(|error| format!("无法解析 {}：{error}", path.display()))?;
            find_node_names_in_items(&syntax.items, &mut names)?;
        }
    }

    match names.as_slice() {
        [] => Ok(None),
        [name] => Ok(Some(name.clone())),
        _ => Err(format!(
            "同一 crate 中发现多个节点声明：{}；每个 cdylib 必须只提供一个节点",
            names.join("、")
        )),
    }
}

/// 在语法树中递归查找节点属性，兼容以后将实现拆分到子模块的结构。
fn find_node_names_in_items(items: &[Item], names: &mut Vec<String>) -> Result<(), String> {
    for item in items {
        if let Some(name) = node_name_in_attributes(item_attributes(item))? {
            names.push(name);
        }
        if let Item::Mod(module) = item
            && let Some((_, nested_items)) = &module.content
        {
            find_node_names_in_items(nested_items, names)?;
        }
    }
    Ok(())
}

/// 取出所有拥有属性的顶层 Rust 项；无属性项无需参与查找。
fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(value) => &value.attrs,
        Item::Enum(value) => &value.attrs,
        Item::ExternCrate(value) => &value.attrs,
        Item::Fn(value) => &value.attrs,
        Item::ForeignMod(value) => &value.attrs,
        Item::Impl(value) => &value.attrs,
        Item::Macro(value) => &value.attrs,
        Item::Mod(value) => &value.attrs,
        Item::Static(value) => &value.attrs,
        Item::Struct(value) => &value.attrs,
        Item::Trait(value) => &value.attrs,
        Item::TraitAlias(value) => &value.attrs,
        Item::Type(value) => &value.attrs,
        Item::Union(value) => &value.attrs,
        Item::Use(value) => &value.attrs,
        _ => &[],
    }
}

/// `node` 属性的参数。发布工具只需要节点名称，但必须接受框架扩展的子配置。
struct NodeAttributeArguments {
    name: LitStr,
}

impl Parse for NodeAttributeArguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let _: Expr = input.parse()?;
        }
        Ok(Self { name })
    }
}

fn node_name_in_attributes(attributes: &[Attribute]) -> Result<Option<String>, String> {
    // 识别 `#[miresidian::node("名称", ...)]`，其他属性一律忽略。
    for attribute in attributes {
        let Some(segment) = attribute.path().segments.last() else {
            continue;
        };
        if segment.ident != "node"
            || (attribute.path().segments.len() > 1
                && attribute
                    .path()
                    .segments
                    .first()
                    .is_none_or(|s| s.ident != "miresidian"))
        {
            continue;
        }
        let name = attribute
            .parse_args::<NodeAttributeArguments>()
            .map_err(|error| format!("无法解析 node 属性的名称参数：{error}"))?;
        return Ok(Some(name.name.value()));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 节点名称解析兼容租约子配置() {
        let item: Item = syn::parse_str(
            r#"
                #[miresidian::node("串口通信", lease(
                    activation = selectable,
                    resource = "port_name",
                    configuration = ["baud_rate", "data_bits"]
                ))]
                struct SerialNode;
            "#,
        )
        .expect("测试属性应符合 Rust 语法");

        assert_eq!(
            node_name_in_attributes(item_attributes(&item))
                .unwrap()
                .as_deref(),
            Some("串口通信")
        );
    }
}

#[cfg(test)]
mod structure_tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn 多节点声明不能静默选择其中一个() {
        let temp = TempDir::new();
        fs::write(temp.0.join("lib.rs"), r#"#[miresidian::node("A")] struct A; mod nested { #[miresidian::node("B")] struct B; }"#).unwrap();
        assert!(
            find_node_name(&temp.0)
                .unwrap_err()
                .contains("多个节点声明")
        );
    }
}
