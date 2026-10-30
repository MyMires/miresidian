//! 结构检查不构建、不加载 DLL，也不校验硬件行为。
use std::{fs, path::Path};

use crate::{discover, model::NodeRelease};

pub(crate) fn check_project(root: &Path, name: Option<&str>) -> Result<Vec<NodeRelease>, String> {
    let metadata = discover::metadata(root)?;
    super::dependencies::validate_manifests(&metadata, root)?;
    let nodes = discover::select_nodes(discover::discover_nodes(&metadata)?, name)?;
    for node in &nodes {
        validate_documents(&node.project_directory, &node.node_name)?;
    }
    Ok(nodes)
}

/// 显示名也用于 Windows 发布目录，必须是可移植的单个路径分量。
pub(crate) fn validate_node_name(name: &str) -> Result<(), String> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|s| matches!(s, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        });
    if name.trim().is_empty()
        || name.ends_with(['.', ' '])
        || reserved
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err(format!("节点名称 {name:?} 不能用作 Windows 发布目录名"));
    }
    Ok(())
}

fn validate_documents(project: &Path, name: &str) -> Result<(), String> {
    for path in [
        project.join("docs").join(format!("{name}.md")),
        project.join("CHANGELOG.md"),
    ] {
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("缺少或无法读取交付文档 {}：{e}", path.display()))?;
        if text.trim().is_empty() {
            return Err(format!("交付文档不能为空：{}", path.display()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 发布名称不能越出目录或使用保留名() {
        for name in [
            "", "..", "../other", "a\\b", "CON", "nul.txt", "COM1", "bad:", "tail.",
        ] {
            assert!(validate_node_name(name).is_err(), "{name}");
        }
        for name in ["设备", "Demo Node", "demo-node", "COM10"] {
            validate_node_name(name).unwrap();
        }
    }

    #[test]
    fn 打包前发现缺失或空白交付文档() {
        let temp = crate::test_support::TempDir::new();
        let node = temp.node();
        validate_documents(&temp.0, &node.node_name).unwrap();
        fs::write(temp.0.join("CHANGELOG.md"), " \n").unwrap();
        assert!(validate_documents(&temp.0, &node.node_name).is_err());
        fs::remove_file(temp.0.join("docs/测试节点.md")).unwrap();
        assert!(validate_documents(&temp.0, &node.node_name).is_err());
    }
}
