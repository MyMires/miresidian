//! Miresidian 节点发布包整理工具。
//!
//! 在节点项目根目录运行 `cargo miresidian-pack` 即构建该项目的 x86 与 x64 发布包；
//! `cargo miresidian-pack x86`、`cargo miresidian-pack x64` 可只构建对应位数。DLL 与 C 头文件按位数放入子目录，节点说明位于
//! `dist/<节点名称>/` 根目录。

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use cargo_metadata::{MetadataCommand, Package, TargetKind};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use syn::{Attribute, Item, LitStr};

/// 一个可发布的 Windows 目标。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WindowsTarget {
    /// Cargo/Rust 使用的目标三元组。
    triple: &'static str,
    /// 发布目录中使用的平台名称。
    package_name: &'static str,
}

/// 用户本次请求的构建范围。
#[derive(Debug, Eq, PartialEq)]
struct PackageRequest {
    /// 需要构建的 Windows 目标；未指定架构时包含 x64 与 x86。
    targets: Vec<WindowsTarget>,
    /// 需要构建的节点显示名；为空时构建全部已发现的节点。
    node_name: Option<String>,
}

/// 一个待发布的节点 crate。
struct NodeRelease {
    /// 交给 Cargo 构建的包名称。
    package: String,
    /// 实例项目 Cargo.toml 所在目录，用于收集发布随附文档。
    project_directory: PathBuf,
    /// 由解析到的 miresidian_abi crate 提供的发布头文件。
    abi_header: PathBuf,
    /// 节点的发布目录与命令筛选名称。
    node_name: String,
    /// cdylib 编译完成后的 DLL 文件名。
    dll_name: String,
    /// 实例项目的语义化版本。
    project_version: String,
    /// 实例声明且经打包器校验的Miresidian 版本。
    miresidian_version: String,
    /// 实例声明且经打包器校验的 ABI 版本。
    abi_version: u32,
}

/// 已复制到发布目录的单个 DLL 及其校验和。
struct ReleaseArtifact {
    platform: &'static str,
    file: String,
    sha256: String,
}

fn main() -> ExitCode {
    match package() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("打包 Miresidian 节点失败：{message}");
            ExitCode::FAILURE
        }
    }
}

/// 构建选定节点、选定平台，并整理为测试平台可直接取用的发布结构。
fn package() -> Result<(), String> {
    let request = selected_request()?;
    let root = env::current_dir().map_err(|error| format!("无法确定当前节点项目目录：{error}"))?;
    let discovered_nodes = discover_nodes(&root)?;
    let nodes = select_nodes(discovered_nodes, request.node_name.as_deref())?;

    for node in nodes {
        let mut artifacts = Vec::with_capacity(request.targets.len());
        for target in &request.targets {
            build_node(&root, *target, &node)?;
            artifacts.push(copy_target_files(&root, *target, &node)?);
        }
        copy_node_release_documents(&root, &node)?;
        write_release_manifest(&root, &node, &artifacts)?;

        println!(
            "已生成可交付的 {} 节点：{}",
            node.node_name,
            root.join("dist").join(&node.node_name).display()
        );
    }
    Ok(())
}

/// 解析打包参数，兼容全部节点、全部位数与单节点、单个位数的组合。
fn selected_request() -> Result<PackageRequest, String> {
    request_from_args(env::args().skip(1))
}

fn request_from_args(
    arguments: impl IntoIterator<Item = String>,
) -> Result<PackageRequest, String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => Ok(PackageRequest {
            targets: vec![windows_x64(), windows_x86()],
            node_name: None,
        }),
        [node_name] if node_name != "x64" && node_name != "x86" => Ok(PackageRequest {
            targets: vec![windows_x64(), windows_x86()],
            node_name: Some(node_name.clone()),
        }),
        [architecture] => Ok(PackageRequest {
            targets: vec![target_from_architecture(architecture)?],
            node_name: None,
        }),
        [architecture, node_name] => Ok(PackageRequest {
            targets: vec![target_from_architecture(architecture)?],
            node_name: Some(node_name.clone()),
        }),
        _ => Err(usage()),
    }
}

fn target_from_architecture(architecture: &str) -> Result<WindowsTarget, String> {
    match architecture {
        "x64" => Ok(windows_x64()),
        "x86" => Ok(windows_x86()),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "用法：cargo miresidian-pack；cargo miresidian-pack x86；cargo miresidian-pack x64".to_owned()
}

fn windows_x64() -> WindowsTarget {
    WindowsTarget {
        triple: "x86_64-pc-windows-gnu",
        package_name: "windows-x64",
    }
}

fn windows_x86() -> WindowsTarget {
    WindowsTarget {
        triple: "i686-pc-windows-gnu",
        package_name: "windows-x86",
    }
}

/// 从 Cargo 工作区自动发现标注了 `#[miresidian::node(...)]` 的 cdylib 节点。
fn discover_nodes(root: &Path) -> Result<Vec<NodeRelease>, String> {
    let metadata = MetadataCommand::new()
        .current_dir(root)
        .exec()
        .map_err(|error| format!("无法读取 Cargo 工作区元数据：{error}"))?;
    let framework = metadata
        .packages
        .iter()
        .find(|package| package.name == "miresidian")
        .ok_or_else(|| "当前节点项目未解析到 miresidian 依赖".to_owned())?;
    let miresidian_version = framework.version.to_string();
    let abi = metadata
        .packages
        .iter()
        .find(|package| package.name == "miresidian_abi")
        .ok_or_else(|| "当前节点项目未解析到 miresidian_abi 依赖".to_owned())?;
    let abi_header = abi
        .manifest_path
        .parent()
        .map(|directory| PathBuf::from(directory.as_str()).join("api/miresidian.h"))
        .filter(|path| path.is_file())
        .ok_or_else(|| "miresidian_abi 包未包含 api/miresidian.h".to_owned())?;
    let framework_abi_version = abi
        .metadata
        .get("miresidian")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("abi_version"))
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| "miresidian_abi 包缺少有效的 ABI 元数据".to_owned())?;
    let mut nodes = Vec::new();

    for member in &metadata.workspace_members {
        let package = metadata
            .packages
            .iter()
            .find(|package| &package.id == member)
            .ok_or_else(|| format!("工作区成员 {member} 不存在于 Cargo 元数据中"))?;
        let manifest_directory = package
            .manifest_path
            .parent()
            .ok_or_else(|| format!("无法确定包 {} 的 Cargo.toml 所在目录", package.name))?;
        let source_directory = PathBuf::from(manifest_directory.as_str()).join("src");
        let Some(node_name) = find_node_name(&source_directory)? else {
            continue;
        };
        let cdylib = package
            .targets
            .iter()
            .find(|target| target.kind.contains(&TargetKind::CDyLib))
            .ok_or_else(|| {
                format!(
                    "节点 {} 标注了 #[miresidian::node]，但 Cargo.toml 未声明 cdylib 产物",
                    package.name
                )
            })?;

        if nodes
            .iter()
            .any(|node: &NodeRelease| node.node_name == node_name)
        {
            return Err(format!("发现重复的节点显示名：{node_name}"));
        }
        let (declared_miresidian_version, abi_version) =
            release_versions(package, &miresidian_version, framework_abi_version)?;
        nodes.push(NodeRelease {
            package: package.name.clone(),
            project_directory: PathBuf::from(manifest_directory.as_str()),
            abi_header: abi_header.clone(),
            node_name,
            dll_name: format!("{}.dll", cdylib.name.replace('-', "_")),
            project_version: package.version.to_string(),
            miresidian_version: declared_miresidian_version,
            abi_version,
        });
    }

    if nodes.is_empty() {
        return Err("工作区中未发现包含 #[miresidian::node(...)] 的 cdylib 节点".to_owned());
    }
    nodes.sort_by(|left, right| left.node_name.cmp(&right.node_name));
    Ok(nodes)
}

/// 读取节点发布声明，并拒绝与当前 Miresidian不一致的版本组合。
fn release_versions(
    package: &Package,
    miresidian_version: &str,
    framework_abi_version: u32,
) -> Result<(String, u32), String> {
    let metadata = package
        .metadata
        .get("miresidian")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("节点 {} 缺少 package.metadata.miresidian", package.name))?;

    let declared_miresidian_version = metadata
        .get("miresidian_version")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("节点 {} 缺少 miresidian_version", package.name))?;
    if declared_miresidian_version != miresidian_version {
        return Err(format!(
            "节点 {} 声明的Miresidian 版本 {} 与当前Miresidian 版本 {} 不一致",
            package.name, declared_miresidian_version, miresidian_version
        ));
    }

    let abi_version = metadata
        .get("miresidian_abi_version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| format!("节点 {} 缺少有效的 miresidian_abi_version", package.name))?;
    if abi_version != framework_abi_version {
        return Err(format!(
            "节点 {} 声明的 ABI 版本 {} 与当前 ABI 版本 {} 不一致",
            package.name, abi_version, framework_abi_version
        ));
    }

    Ok((declared_miresidian_version.to_owned(), abi_version))
}

/// 按节点显示名筛选；没有筛选条件时保留全部发现的节点。
fn select_nodes(
    nodes: Vec<NodeRelease>,
    requested_name: Option<&str>,
) -> Result<Vec<NodeRelease>, String> {
    let Some(requested_name) = requested_name else {
        return Ok(nodes);
    };

    let available_names = nodes
        .iter()
        .map(|node| node.node_name.as_str())
        .collect::<Vec<_>>()
        .join("、");
    let selected = nodes
        .into_iter()
        .filter(|node| node.node_name == requested_name)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(format!(
            "未找到名称为“{requested_name}”的节点；当前可用节点：{available_names}"
        ));
    }
    Ok(selected)
}

/// 调用 Cargo 的普通 build 子命令，复用仓库 `.cargo/config.toml` 中的链接器配置。
fn build_node(root: &Path, target: WindowsTarget, node: &NodeRelease) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
        .args([
            "build",
            "--package",
            node.package.as_str(),
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

/// 复制某个位数测试平台所需的 DLL 和 C 头文件。
fn copy_target_files(
    root: &Path,
    target: WindowsTarget,
    node: &NodeRelease,
) -> Result<ReleaseArtifact, String> {
    let release_dir = root.join("target").join(target.triple).join("release");
    let source_dll = release_dir.join(&node.dll_name);
    let destination = root
        .join("dist")
        .join(&node.node_name)
        .join(target.package_name);

    if !source_dll.is_file() {
        return Err(format!("未找到预期 DLL：{}", source_dll.display()));
    }

    fs::create_dir_all(&destination).map_err(|error| format!("无法创建发布目录：{error}"))?;
    remove_legacy_documentation(&destination, &node.node_name)?;
    let destination_dll = destination.join(&node.dll_name);
    copy_file(&source_dll, &destination_dll)?;
    copy_file(&node.abi_header, &destination.join("miresidian.h"))?;
    Ok(ReleaseArtifact {
        platform: target.package_name,
        file: node.dll_name.clone(),
        sha256: sha256_file(&destination_dll)?,
    })
}

/// 计算发布 DLL 的 SHA-256，供部署端校验文件完整性。
fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let bytes = file
            .read(&mut buffer)
            .map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
        if bytes == 0 {
            break;
        }
        hasher.update(&buffer[..bytes]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 生成与当前 DLL 集合对应的发布元数据。
fn write_release_manifest(
    root: &Path,
    node: &NodeRelease,
    artifacts: &[ReleaseArtifact],
) -> Result<(), String> {
    let destination = root.join("dist").join(&node.node_name);
    let document = json!({
        "format_version": 1,
        "node": {
            "name": node.node_name,
            "package": node.package,
            "version": node.project_version,
        },
        "miresidian": { "version": node.miresidian_version },
        "abi": { "version": node.abi_version },
        "source": { "git_revision": git_revision(root) },
        "artifacts": artifacts.iter().map(|artifact| json!({
            "platform": artifact.platform,
            "file": artifact.file,
            "sha256": artifact.sha256,
        })).collect::<Vec<_>>(),
    });
    let content = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("无法序列化发布清单：{error}"))?;
    fs::write(destination.join("manifest.json"), format!("{content}\n"))
        .map_err(|error| format!("无法写入发布清单：{error}"))
}

/// 读取发布时的 Git 提交；非 Git 工作区仍可打包，但会明确标为 unknown。
fn git_revision(root: &Path) -> String {
    Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|revision| revision.trim().to_owned())
        .filter(|revision| !revision.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// 清理旧版位数目录中的说明，确保发布包只保留根目录的一份说明文件。
fn remove_legacy_documentation(destination: &Path, node_name: &str) -> Result<(), String> {
    let legacy_documentation = destination.join(format!("{node_name}.md"));
    if legacy_documentation.is_file() {
        fs::remove_file(&legacy_documentation).map_err(|error| {
            format!(
                "无法清理旧版说明 {}：{error}",
                legacy_documentation.display()
            )
        })?;
    }
    Ok(())
}

/// 复制节点部署说明与修订记录，避免在位数目录中出现重复文档。
fn copy_node_release_documents(root: &Path, node: &NodeRelease) -> Result<(), String> {
    let destination = root.join("dist").join(&node.node_name);
    fs::create_dir_all(&destination).map_err(|error| format!("无法创建发布目录：{error}"))?;
    copy_file(
        &node
            .project_directory
            .join("docs")
            .join(format!("{}.md", node.node_name)),
        &destination.join(format!("{}.md", node.node_name)),
    )?;
    copy_file(
        &node.project_directory.join("CHANGELOG.md"),
        &destination.join("CHANGELOG.md"),
    )
}

/// 从节点源码中读取节点属性宏的显示名；不含节点属性的 crate 返回 `None`。
fn find_node_name(source_directory: &Path) -> Result<Option<String>, String> {
    let mut pending = vec![source_directory.to_owned()];

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("无法读取节点源码目录 {}：{error}", directory.display()))?
        {
            let entry = entry.map_err(|error| format!("无法读取节点源码条目：{error}"))?;
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
                return Ok(Some(name));
            }
        }
    }

    Ok(None)
}

/// 在语法树中递归查找节点属性，兼容以后将实现拆分到子模块的结构。
fn find_node_name_in_items(items: &[Item]) -> Result<Option<String>, String> {
    for item in items {
        if let Some(name) = node_name_in_attributes(item_attributes(item))? {
            return Ok(Some(name));
        }
        if let Item::Mod(module) = item
            && let Some((_, nested_items)) = &module.content
            && let Some(name) = find_node_name_in_items(nested_items)?
        {
            return Ok(Some(name));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn 默认请求构建全部节点的两个位数() {
        let request = request_from_args(arguments(&[])).expect("默认参数应可解析");

        assert_eq!(request.node_name, None);
        assert_eq!(request.targets, vec![windows_x64(), windows_x86()]);
    }

    #[test]
    fn 节点名称默认构建两个位数() {
        let request = request_from_args(arguments(&["串口通信"])).expect("节点名称参数应可解析");

        assert_eq!(request.node_name.as_deref(), Some("串口通信"));
        assert_eq!(request.targets, vec![windows_x64(), windows_x86()]);
    }

    #[test]
    fn 位数与节点名称可以组合() {
        let request =
            request_from_args(arguments(&["x86", "串口通信"])).expect("位数和节点名称参数应可解析");

        assert_eq!(request.node_name.as_deref(), Some("串口通信"));
        assert_eq!(request.targets, vec![windows_x86()]);
    }
}
