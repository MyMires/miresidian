//! 调用 Cargo 构建选定的 Windows DLL。
use crate::model::{NodeRelease, WindowsTarget};
use std::{env, path::Path, process::Command};

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
