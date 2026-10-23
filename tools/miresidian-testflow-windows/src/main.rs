//! Miresidian Windows TestFlow BAT 生成工具。
//!
//! 在节点项目根目录运行 `cargo miresidian-testflow-windows <流程.testflow>`。工具会确认
//! 当前包声明了 `cdylib`，并在流程文件旁生成同名 `.bat`，供 Windows 上的 PowerShell
//! 调用器执行。它只生成入口，不解释 `.testflow` 内容、不构建 DLL，也不直接访问硬件。
//!
use cargo_metadata::{MetadataCommand, TargetKind};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
fn main() -> ExitCode {
    match run() {
        Ok(p) => {
            println!("已生成：{}", p.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("生成失败：{e}");
            ExitCode::FAILURE
        }
    }
}
/// 读取节点项目元数据，并依据 cdylib 名称生成与流程同名的 BAT 文件。
fn run() -> Result<PathBuf, String> {
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 1 {
        return Err("用法：cargo miresidian-testflow-windows <流程.testflow>".into());
    }
    let root = env::current_dir().map_err(|e| e.to_string())?;
    let flow = root.join(&a[0]);
    validate_testflow_extension(&flow)?;
    let text = fs::read_to_string(&flow).map_err(|e| e.to_string())?;
    if text.trim().is_empty() {
        return Err("流程文件不能为空".into());
    }
    let m = MetadataCommand::new()
        .current_dir(&root)
        .no_deps()
        .exec()
        .map_err(|e| e.to_string())?;
    let p = m.root_package().ok_or("请在节点项目根目录运行")?;
    let t = p
        .targets
        .iter()
        .find(|t| t.kind.contains(&TargetKind::CDyLib))
        .ok_or("当前项目未声明 cdylib")?;
    let name = flow
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or("流程文件名无效")?;
    let out = flow.with_extension("bat");
    fs::write(&out, batch_body(name, &t.name)).map_err(|e| e.to_string())?;
    Ok(out)
}

/// 防止把旧 `.flow` 或任意文本文件误当作当前 TestFlow 入口。
fn validate_testflow_extension(flow: &Path) -> Result<(), String> {
    if flow.extension().and_then(|extension| extension.to_str()) == Some("testflow") {
        Ok(())
    } else {
        Err("流程文件扩展名必须是 .testflow".into())
    }
}

/// 生成 BAT 内容；DLL 路径相对于节点项目的 `tools/` 目录固定为 `../dist`。
fn batch_body(flow_name: &str, cdylib_name: &str) -> String {
    format!(
        "@echo off\r\nsetlocal\r\npushd \"%~dp0\"\r\nset \"MIRESIDIAN_FLOW_FILE={flow_name}\"\r\nset \"MIRESIDIAN_DLL_NAME={}.dll\"\r\nset \"MIRESIDIAN_DLL_PATH=..\\dist\"\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -File \"invoke_miresidian_dll.ps1\" -FromEnvironment -Pretty\r\npause\r\npopd\r\nendlocal\r\n",
        cdylib_name.replace('-', "_")
    )
}

#[cfg(test)]
mod tests {
    use super::{batch_body, validate_testflow_extension};
    use std::path::Path;

    #[test]
    fn only_testflow_files_are_accepted() {
        assert!(validate_testflow_extension(Path::new("serial_smoke.testflow")).is_ok());
        assert!(validate_testflow_extension(Path::new("serial_smoke.flow")).is_err());
    }

    #[test]
    fn windows_batch_uses_flow_name_and_cdylib_name() {
        let batch = batch_body("serial_smoke.testflow", "serial-node");
        assert!(batch.contains("MIRESIDIAN_FLOW_FILE=serial_smoke.testflow"));
        assert!(batch.contains("MIRESIDIAN_DLL_NAME=serial_node.dll"));
        assert!(batch.contains("MIRESIDIAN_DLL_PATH=..\\dist"));
    }
}
