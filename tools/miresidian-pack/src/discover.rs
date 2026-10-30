//! Cargo 元数据与待交付节点发现。
use crate::{model::NodeRelease, source::find_node_name};
use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId, TargetKind};
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// 从 Cargo 工作区自动发现标注了 `#[miresidian::node(...)]` 的 cdylib 节点。
pub(crate) fn discover_nodes(
    metadata: &cargo_metadata::Metadata,
) -> Result<Vec<NodeRelease>, String> {
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

        crate::validate::validate_node_name(&node_name)?;
        let dependencies = dependencies_of(metadata, member)?;
        crate::validate::validate_resolved_sources(&dependencies)?;
        let framework = unique_package(&dependencies, "miresidian")?;
        let abi = unique_package(&dependencies, "miresidian_abi")?;
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
            .and_then(|m| m.get("abi_version"))
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| "miresidian_abi 包缺少有效的 ABI 元数据".to_owned())?;
        if nodes
            .iter()
            .any(|node: &NodeRelease| node.node_name == node_name)
        {
            return Err(format!("发现重复的节点显示名：{node_name}"));
        }
        nodes.push(NodeRelease {
            package: package.name.clone(),
            project_directory: PathBuf::from(manifest_directory.as_str()),
            abi_header,
            target_directory: PathBuf::from(metadata.target_directory.as_str()),
            node_name,
            dll_name: format!("{}.dll", cdylib.name.replace('-', "_")),
            project_version: package.version.to_string(),
            miresidian_version: framework.version.to_string(),
            abi_version: framework_abi_version,
        });
    }

    if nodes.is_empty() {
        return Err("工作区中未发现包含 #[miresidian::node(...)] 的 cdylib 节点".to_owned());
    }
    nodes.sort_by(|left, right| left.node_name.cmp(&right.node_name));
    Ok(nodes)
}

/// 按节点显示名筛选；没有筛选条件时保留全部发现的节点。
pub(crate) fn select_nodes(
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

/// 使用锁文件读取真实依赖；检查与打包均不得悄悄更新依赖版本。
pub(crate) fn metadata(root: &Path) -> Result<Metadata, String> {
    MetadataCommand::new()
        .current_dir(root)
        .other_options(vec!["--locked".into()])
        .exec()
        .map_err(|error| {
            format!("无法读取锁定的 Cargo 元数据；请先运行 cargo generate-lockfile：{error}")
        })
}

/// 分别遍历每个节点的依赖图，避免从整个工作区任取一个框架版本。
fn dependencies_of<'a>(
    metadata: &'a Metadata,
    member: &PackageId,
) -> Result<Vec<&'a Package>, String> {
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or("Cargo 元数据缺少已解析依赖图")?;
    let mut pending = VecDeque::from([member.clone()]);
    let mut visited = HashSet::new();
    let mut packages = Vec::new();
    while let Some(id) = pending.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let package = metadata
            .packages
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("依赖图中的 {id} 缺少包元数据"))?;
        packages.push(package);
        let node = resolve
            .nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or_else(|| format!("依赖图中的 {id} 缺少解析节点"))?;
        pending.extend(node.deps.iter().map(|d| d.pkg.clone()));
    }
    Ok(packages)
}

fn unique_package<'a>(packages: &[&'a Package], name: &str) -> Result<&'a Package, String> {
    let matches = packages
        .iter()
        .filter(|p| p.name == name)
        .copied()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [package] => Ok(package),
        [] => Err(format!("节点未解析到 {name} 依赖")),
        _ => Err(format!(
            "节点同时解析到多个 {name} 版本，无法唯一确定交付契约"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn 工作区不同节点按自己的依赖图确定框架版本() {
        // 只构造元数据图，不生成锁文件、不构建任何本地框架替代节点。
        let package = |name: &str, version: &str, id: &str| {
            json!({
                "name":name, "version":version, "id":id, "dependencies":[],
                "targets":[], "features":{}, "manifest_path":format!("/fixture/{id}/Cargo.toml"),
            })
        };
        let node = |id: &str, dependency: Option<&str>| {
            json!({
                "id":id, "dependencies":dependency.into_iter().collect::<Vec<_>>(),
                "deps":dependency.into_iter().map(|pkg| json!({"name":"miresidian", "pkg":pkg, "dep_kinds":[]})).collect::<Vec<_>>(),
                "features":[],
            })
        };
        let metadata: Metadata = serde_json::from_value(json!({
            "packages":[package("miresidian", "0.13.0", "old"), package("miresidian", "0.14.0", "new"), package("a", "0.1.0", "a"), package("b", "0.1.0", "b")],
            "workspace_members":["a", "b"], "workspace_root":"/fixture", "target_directory":"/fixture/target", "version":1,
            "resolve":{"root":null, "nodes":[node("a", Some("new")), node("b", Some("old")), node("old", None), node("new", None)]},
        })).unwrap();
        let a = dependencies_of(&metadata, &metadata.workspace_members[0]).unwrap();
        let b = dependencies_of(&metadata, &metadata.workspace_members[1]).unwrap();
        assert_eq!(
            unique_package(&a, "miresidian")
                .unwrap()
                .version
                .to_string(),
            "0.14.0"
        );
        assert_eq!(
            unique_package(&b, "miresidian")
                .unwrap()
                .version
                .to_string(),
            "0.13.0"
        );
        assert!(
            unique_package(&metadata.packages.iter().collect::<Vec<_>>(), "miresidian").is_err()
        );
    }
}
