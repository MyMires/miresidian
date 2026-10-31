//! 管理命令委托 Cargo，使用其安装根目录、版本替换和安装记录；不改项目文件。
use std::{env, ffi::OsString, process::Command};
use toml::Value;

pub(super) const HELP: &str = "工具管理（mires 为等价入口）：\n  cargo miresidian install --full [--pack-version X.Y.Z] [--testflow-version X.Y.Z] [Cargo 参数]\n  cargo miresidian install <工具> [--version X.Y.Z] [Cargo 参数]\n  cargo miresidian update --full [Cargo 参数]\n  cargo miresidian update <工具> [Cargo 参数]\n  cargo miresidian use <工具> <X.Y.Z> [Cargo 参数]\n  cargo miresidian list [--root <目录>]\n工具可用别名或完整 crate 名称；--full 包含全部登记的后端工具。\n未指定版本时由 Cargo 选择最新正式版，指定版本时替换该目录中的现有版本。\nCargo 参数（如 --locked、--offline、--root、--force）遵循原生语义；其他参数可放在 -- 后。\n默认安装到 Cargo 用户目录；不创建项目内工具目录，不需要 Python。\nCLI 本体：cargo install miresidian_cli [--version X.Y.Z]。";

#[derive(Debug)]
pub(in crate::app) enum Action {
    Help,
    Cargo(Vec<OsString>),
}

pub(super) struct Tool {
    pub(super) alias: String,
    pub(super) package: String,
}

pub(super) fn tools() -> Result<Vec<Tool>, String> {
    let manifest: Value =
        toml::from_str(super::super::routing::TOOLSET).map_err(|e| e.to_string())?;
    manifest["tools"]
        .as_array()
        .ok_or("工具清单无效")?
        .iter()
        .map(|tool| {
            Ok(Tool {
                alias: tool["alias"].as_str().ok_or("工具缺少别名")?.to_owned(),
                package: tool["package"].as_str().ok_or("工具缺少包名")?.to_owned(),
            })
        })
        .collect()
}

pub(in crate::app) fn execute(action: Action) -> Result<i32, String> {
    match action {
        Action::Help => {
            println!("{HELP}");
            Ok(0)
        }
        Action::Cargo(args) => {
            let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let status = Command::new(cargo)
                .args(&args)
                .status()
                .map_err(|e| format!("无法启动 Cargo：{e}"))?;
            if !status.success() {
                eprintln!(
                    "Cargo 工具管理失败；请按上方 Cargo 输出核对。批量操作可能部分完成，可用 cargo miresidian list 查看；不会自动回滚或重试。"
                );
            }
            Ok(status.code().unwrap_or(1))
        }
    }
}
