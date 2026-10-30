//! 发布清单、文件摘要与来源修订。
use crate::model::{NodeRelease, ReleaseArtifact};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path, process::Command};

/// 计算发布 DLL 的 SHA-256，供部署端校验文件完整性。
pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
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
pub(crate) fn write_release_manifest(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn 清单保持既有字段与版本来源() {
        let temp = TempDir::new();
        let node = temp.node();
        fs::create_dir_all(temp.0.join("dist/测试节点")).unwrap();
        let artifacts = [ReleaseArtifact {
            platform: "windows-x64",
            file: "test_node.dll".into(),
            sha256: "abc123".into(),
        }];
        write_release_manifest(&temp.0, &node, &artifacts).unwrap();
        let value: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(temp.0.join("dist/测试节点/manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            value,
            json!({
                "format_version": 1,
                "node": {"name":"测试节点", "package":"test-node", "version":"0.1.0"},
                "miresidian": {"version":"0.14.0"}, "abi":{"version":7},
                "source":{"git_revision":"unknown"},
                "artifacts":[{"platform":"windows-x64", "file":"test_node.dll", "sha256":"abc123"}],
            })
        );
    }
}
