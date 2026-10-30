//! 构建、收集及清单生成的交付流程。
use crate::{
    artifacts::{copy_node_release_documents, copy_target_files},
    build::build_node,
    manifest::write_release_manifest,
};
use std::env;

/// 构建选定节点、选定平台，并整理为测试平台可直接取用的发布结构。
pub(crate) fn package(request: crate::model::PackageRequest) -> Result<(), String> {
    let root = env::current_dir().map_err(|error| format!("无法确定当前节点项目目录：{error}"))?;
    let nodes = crate::validate::check_project(&root, request.node_name.as_deref())?;

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
