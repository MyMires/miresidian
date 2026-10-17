//! Miresidian 驱动发布包整理工具。
//!
//! 此工具由 `cargo dll` 与 `cargo dll32` 调用：先构建目标 DLL，再将 DLL、C 头文件和
//! 使用说明放入项目根目录的 `dist/serial/<平台>/`，使其可以直接交付给测试平台。

use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
use syn::{Attribute, Item, LitStr};

/// 一个可发布的 Windows 目标。
#[derive(Clone, Copy)]
struct WindowsTarget {
    /// Cargo/Rust 使用的目标三元组。
    triple: &'static str,
    /// 发布目录中使用的平台名称。
    package_name: &'static str,
}

/// 一个待发布的节点 crate。
struct DriverRelease {
    /// 交给 Cargo 构建的包名称。
    package: &'static str,
    /// 驱动源码根目录；发布工具会在其中查找 `#[miresidian::node(...)]`。
    source_directory: &'static str,
}

fn main() -> ExitCode {
    match package() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("打包 Miresidian 串口通信驱动失败：{message}");
            ExitCode::FAILURE
        }
    }
}

/// 构建选定平台并整理可交付文件。
fn package() -> Result<(), String> {
    let target = selected_target()?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let driver = serial_driver();
    let node_name = find_node_name(&root.join(driver.source_directory))?;

    build_driver(&root, target, driver)?;
    copy_release_files(&root, target, &node_name)?;

    println!(
        "已生成可交付的串口通信驱动：{}",
        root.join("dist").join(&node_name).join(target.package_name).display()
    );
    Ok(())
}

/// 当前实例库的 Cargo 包信息；节点显示名始终从属性宏中读取，不在此处重复维护。
fn serial_driver() -> DriverRelease {
    DriverRelease {
        package: "miresidian-serial",
        source_directory: "constellation/serial/src",
    }
}

/// 读取 `x64` 或 `x86` 参数，避免把平台选择藏在脚本中。
fn selected_target() -> Result<WindowsTarget, String> {
    match env::args().nth(1).as_deref() {
        Some("x64") => Ok(WindowsTarget {
            triple: "x86_64-pc-windows-gnu",
            package_name: "windows-x64",
        }),
        Some("x86") => Ok(WindowsTarget {
            triple: "i686-pc-windows-gnu",
            package_name: "windows-x86",
        }),
        _ => Err("用法：cargo package_driver -- <x64|x86>".to_owned()),
    }
}

/// 调用 Cargo 的普通 build 子命令，复用仓库 `.cargo/config.toml` 中的链接器配置。
fn build_driver(root: &Path, target: WindowsTarget, driver: DriverRelease) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
        .args([
            "build",
            "--package",
            driver.package,
            "--release",
            "--target",
            target.triple,
        ])
        .status()
        .map_err(|error| format!("无法启动 Cargo：{error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Cargo 构建失败，退出码：{status}"))
    }
}

/// 仅复制测试平台部署所需的文件，避免把 `target/` 中间产物交付出去。
fn copy_release_files(root: &Path, target: WindowsTarget, node_name: &str) -> Result<(), String> {
    let release_dir = root.join("target").join(target.triple).join("release");
    let source_dll = release_dir.join("miresidian_serial.dll");
    let driver_directory = root.join("dist").join(node_name);
    let destination = driver_directory.join(target.package_name);

    if !source_dll.is_file() {
        return Err(format!("未找到预期 DLL：{}", source_dll.display()));
    }

    fs::create_dir_all(&destination)
        .map_err(|error| format!("无法创建发布目录：{error}"))?;
    copy_file(&source_dll, &destination.join("miresidian_serial.dll"))?;
    copy_file(
        &root.join("api/miresidian.h"),
        &destination.join("miresidian.h"),
    )?;
    copy_file(
        &root.join("docs/drivers").join(format!("{node_name}.md")),
        &destination.join(format!("{node_name}.md")),
    )?;
    Ok(())
}

/// 从驱动源码中读取节点属性宏的显示名。
fn find_node_name(source_directory: &Path) -> Result<String, String> {
    let mut pending = vec![source_directory.to_owned()];

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("无法读取驱动源码目录 {}：{error}", directory.display()))?
        {
            let entry = entry.map_err(|error| format!("无法读取驱动源码条目：{error}"))?;
            let path = entry.path();
            if path.is_dir() {
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
            if let Some(name) = find_node_name_in_items(&syntax.items)? {
                return Ok(name);
            }
        }
    }

    Err(format!(
        "在 {} 中未找到 #[miresidian::node(\"节点名称\")]",
        source_directory.display()
    ))
}

/// 在语法树中递归查找节点属性，兼容以后将实现拆分到子模块的结构。
fn find_node_name_in_items(items: &[Item]) -> Result<Option<String>, String> {
    for item in items {
        if let Some(name) = node_name_in_attributes(item_attributes(item))? {
            return Ok(Some(name));
        }
        if let Item::Mod(module) = item {
            if let Some((_, nested_items)) = &module.content {
                if let Some(name) = find_node_name_in_items(nested_items)? {
                    return Ok(Some(name));
                }
            }
        }
    }
    Ok(None)
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

/// 识别 `#[miresidian::node("名称")]`，其他属性一律忽略。
fn node_name_in_attributes(attributes: &[Attribute]) -> Result<Option<String>, String> {
    for attribute in attributes {
        let Some(segment) = attribute.path().segments.last() else {
            continue;
        };
        if segment.ident != "node" {
            continue;
        }
        let name = attribute
            .parse_args::<LitStr>()
            .map_err(|error| format!("无法解析 node 属性的名称参数：{error}"))?;
        return Ok(Some(name.value()));
    }
    Ok(None)
}

/// 复制文件，并把路径包含在错误中，方便定位部署问题。
fn copy_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::copy(source, destination).map(|_| ()).map_err(|error| {
        format!(
            "无法复制 {} 到 {}：{error}",
            source.display(),
            destination.display()
        )
    })
}
