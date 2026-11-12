//! 静态提取 operation 方法的字面 Rustdoc 与声明；不推测展开后的平台契约。
use quote::ToTokens;
use std::{
    fs,
    path::{Path, PathBuf},
};
use syn::{Attribute, Expr, ImplItem, Item, Lit, Meta};

pub(super) struct Method {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) declaration: String,
    pub(super) documentation: String,
    pub(super) conditions: Vec<String>,
}

/// 排序后读取整个源码目录，包含私有节点方法及拆分文件中的 impl。
pub(super) fn read_methods(source_directory: &Path) -> Result<Vec<Method>, String> {
    let mut files = Vec::new();
    source_files(source_directory, &mut files)?;
    files.sort();
    let mut methods = Vec::new();
    for path in files {
        let text =
            fs::read_to_string(&path).map_err(|e| format!("无法读取 {}：{e}", path.display()))?;
        let syntax = syn::parse_file(&text)
            .map_err(|e| format!("无法解析业务文档 {}：{e}", path.display()))?;
        // 路径以 src/ 开头，不输出机器特定的绝对工作区目录。
        let relative = Path::new("src").join(
            path.strip_prefix(source_directory)
                .map_err(|e| e.to_string())?,
        );
        collect(
            &syntax.items,
            &relative,
            "",
            &conditions(&syntax.attrs),
            &mut methods,
        );
    }
    Ok(methods)
}

fn source_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|e| format!("无法读取源码目录 {}：{e}", directory.display()))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            source_files(&path, files)?;
        } else if kind.is_file() && path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn collect(
    items: &[Item],
    path: &Path,
    module: &str,
    inherited: &[String],
    methods: &mut Vec<Method>,
) {
    for item in items {
        match item {
            Item::Mod(item) => {
                if let Some((_, nested)) = &item.content {
                    let mut parent = inherited.to_vec();
                    parent.extend(conditions(&item.attrs));
                    collect(
                        nested,
                        path,
                        &format!("{module}{}::", item.ident),
                        &parent,
                        methods,
                    );
                }
            }
            Item::Impl(item) if item.trait_.is_none() => {
                let mut parent = inherited.to_vec();
                parent.extend(conditions(&item.attrs));
                for method in &item.items {
                    let ImplItem::Fn(method) = method else {
                        continue;
                    };
                    if !method.attrs.iter().any(is_operation) {
                        continue;
                    }
                    let mut declaration = String::new();
                    for attr in method.attrs.iter().filter(|a| !a.path().is_ident("doc")) {
                        declaration.push_str(&attr.to_token_stream().to_string());
                        declaration.push('\n');
                    }
                    let visibility = &method.vis;
                    let signature = &method.sig;
                    declaration.push_str(&quote::quote!(#visibility #signature;).to_string());
                    methods.push(Method {
                        name: format!(
                            "{module}{}::{}",
                            item.self_ty.to_token_stream(),
                            method.sig.ident
                        ),
                        path: path.to_owned(),
                        declaration,
                        documentation: documentation(&method.attrs),
                        conditions: parent.clone(),
                    });
                }
            }
            _ => {}
        }
    }
}

fn is_operation(attr: &Attribute) -> bool {
    let path = attr.path();
    path.is_ident("operation")
        || (path.segments.len() == 2
            && path.segments[0].ident == "miresidian"
            && path.segments[1].ident == "operation")
}

fn conditions(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
        .map(|a| a.to_token_stream().to_string())
        .collect()
}

fn documentation(attrs: &[Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs.iter().filter(|a| a.path().is_ident("doc")) {
        if let Meta::NameValue(value) = &attr.meta {
            if let Expr::Lit(literal) = &value.value
                && let Lit::Str(text) = &literal.lit
            {
                lines.extend(
                    text.value()
                        .lines()
                        .map(|line| line.strip_prefix(' ').unwrap_or(line).to_owned()),
                );
            } else {
                lines.push(format!(
                    "未展开的动态文档表达式：`{}`。",
                    value.value.to_token_stream()
                ));
            }
        }
    }
    if lines.iter().all(|line| line.trim().is_empty()) {
        "此方法尚未提供文档注释。".into()
    } else {
        lines.join("\n")
    }
}
