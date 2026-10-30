//! 收集 DLL、ABI 头文件与交付文档。
use crate::{
    manifest::sha256_file,
    model::{NodeRelease, ReleaseArtifact, WindowsTarget},
};
use std::{fs, path::Path};

/// 复制某个位数测试平台所需的 DLL 和 C 头文件。
pub(crate) fn copy_target_files(
    root: &Path,
    target: WindowsTarget,
    node: &NodeRelease,
) -> Result<ReleaseArtifact, String> {
    let release_dir = node.target_directory.join(target.triple).join("release");
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
pub(crate) fn copy_node_release_documents(root: &Path, node: &NodeRelease) -> Result<(), String> {
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
    use crate::{cli::windows_x64, test_support::TempDir};

    #[test]
    fn 使用实际产物目录并保持交付结构和文件内容() {
        let temp = TempDir::new();
        let node = temp.node();
        let target = windows_x64();
        let release = node.target_directory.join(target.triple).join("release");
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join(&node.dll_name), b"abc").unwrap();
        let destination = temp.0.join("dist/测试节点/windows-x64");
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("测试节点.md"), "旧说明").unwrap();

        let artifact = copy_target_files(&temp.0, target, &node).unwrap();
        copy_node_release_documents(&temp.0, &node).unwrap();
        assert_eq!(fs::read(destination.join("test_node.dll")).unwrap(), b"abc");
        assert_eq!(
            fs::read_to_string(destination.join("miresidian.h")).unwrap(),
            "ABI header"
        );
        assert!(!destination.join("测试节点.md").exists());
        assert!(temp.0.join("dist/测试节点/测试节点.md").is_file());
        assert!(temp.0.join("dist/测试节点/CHANGELOG.md").is_file());
        assert_eq!(
            artifact.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
