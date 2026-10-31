//! 调用 Cargo 构建选定的 Windows DLL。
use crate::model::{NodeRelease, WindowsTarget};
use std::{env, path::Path, process::Command};

/// 独立构建保留 Cargo 产物，不生成 dist。
pub(crate) fn build(request: crate::model::PackageRequest) -> Result<(), String> {
    let root = env::current_dir().map_err(|e| e.to_string())?;
    let nodes = crate::validate::check_project(&root, request.node_name.as_deref())?;
    preflight(&request.targets)?;
    for node in nodes {
        for target in &request.targets {
            build_node(&root, *target, &node)?;
        }
    }
    Ok(())
}

pub(crate) fn preflight(targets: &[WindowsTarget]) -> Result<(), String> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    for target in targets {
        let output = Command::new(&rustc)
            .args(["--print", "target-libdir", "--target", target.triple])
            .output()
            .map_err(|e| format!("无法检查 Rust 目标：{e}"))?;
        let dir = String::from_utf8_lossy(&output.stdout);
        let installed = output.status.success()
            && std::fs::read_dir(dir.trim()).is_ok_and(|entries| {
                entries
                    .flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with("libcore-"))
            });
        if !installed {
            return Err(format!(
                "缺少 Rust 目标 {}；请执行 rustup target add {}，并配置对应 Windows GNU 链接器",
                target.triple, target.triple
            ));
        }
    }
    Ok(())
}

/// 调用 Cargo 的普通 build 子命令，复用仓库 `.cargo/config.toml` 中的链接器配置。
pub(crate) fn build_node(
    root: &Path,
    target: WindowsTarget,
    node: &NodeRelease,
) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
        .args([
            "build",
            "--locked",
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
        Err(format!(
            "Cargo 构建失败，退出码：{status}；请核对 {} 的链接器配置和依赖",
            target.triple
        ))
    }
}
