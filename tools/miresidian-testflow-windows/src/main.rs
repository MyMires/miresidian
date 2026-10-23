use cargo_metadata::{MetadataCommand, TargetKind};
use std::{env, fs, path::PathBuf, process::ExitCode};
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
fn run() -> Result<PathBuf, String> {
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 1 {
        return Err("用法：cargo miresidian-testflow-windows <流程.testflow>".into());
    }
    let root = env::current_dir().map_err(|e| e.to_string())?;
    let flow = root.join(&a[0]);
    if flow.extension().and_then(|x| x.to_str()) != Some("testflow") {
        return Err("流程文件扩展名必须是 .testflow".into());
    }
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
    let body = format!(
        "@echo off\r\nsetlocal\r\npushd \"%~dp0\"\r\nset \"MIRESIDIAN_FLOW_FILE={name}\"\r\nset \"MIRESIDIAN_DLL_NAME={}.dll\"\r\nset \"MIRESIDIAN_DLL_PATH=..\\dist\"\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -File \"invoke_miresidian_dll.ps1\" -FromEnvironment -Pretty\r\npause\r\npopd\r\nendlocal\r\n",
        t.name.replace("-", "_")
    );
    fs::write(&out, body).map_err(|e| e.to_string())?;
    Ok(out)
}
