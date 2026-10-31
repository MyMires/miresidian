//! Miresidian Windows TestFlow 自包含 BAT 生成工具。
//!
//! 在节点项目根目录运行 `cargo miresidian-testflow-windows <流程.testflow>`。工具会确认
//! 当前包声明了 `cdylib`，并在流程文件旁生成同名 `.bat`，供 Windows 上的 PowerShell
//! 执行。BAT 以 `-EncodedCommand` 内嵌框架维护的流程执行器，不依赖节点项目中的 `.ps1`；它不构建 DLL，也不直接访问硬件。
//!
use base64::{Engine as _, engine::general_purpose::STANDARD};
use cargo_metadata::{MetadataCommand, TargetKind};
mod flow;
use std::{env, fs, path::Path, process::ExitCode};
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("生成失败：{e}");
            ExitCode::FAILURE
        }
    }
}
/// 读取节点项目元数据，并依据 cdylib 名称生成与流程同名的 BAT 文件。
fn run() -> Result<(), String> {
    let a = cargo_subcommand_arguments(env::args().skip(1));
    match a.as_slice() {
        [flag] if flag == "--help" || flag == "-h" => {
            println!(
                "用法：cargo miresidian-testflow-windows [--check] <流程.testflow>\n--check 仅检查语法和基本调用顺序，不生成 BAT\n--help|-h 帮助\n--version|-V 工具版本\n本工具生成 Windows BAT；DLL 须提前打包，BAT 用 Windows PowerShell 执行。"
            );
            return Ok(());
        }
        [flag] if flag == "--version" || flag == "-V" => {
            println!("miresidian_testflow_windows {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    };
    let (check, file) = match a.as_slice() {
        [file] if !file.starts_with('-') => (false, file),
        [flag, file] if flag == "--check" && !file.starts_with('-') => (true, file),
        _ => {
            return Err("用法：cargo miresidian-testflow-windows [--check] <流程.testflow>".into());
        }
    };
    let root = env::current_dir().map_err(|e| e.to_string())?;
    let flow = root.join(file);
    validate_testflow_extension(&flow)?;
    let text = fs::read_to_string(&flow).map_err(|e| e.to_string())?;
    if text.trim().is_empty() {
        return Err("流程文件不能为空".into());
    }
    flow::validate(&text)?;
    if check {
        println!("流程检查通过：{}", flow.display());
        return Ok(());
    }
    let m = MetadataCommand::new()
        .current_dir(&root)
        .no_deps()
        .other_options(vec!["--locked".into()])
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
    println!("已生成：{}", out.display());
    Ok(())
}

/// Cargo 调用外部子命令时会额外传入子命令名；直接运行二进制时则不会。
fn cargo_subcommand_arguments(arguments: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut arguments = arguments.into_iter().collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == "miresidian-testflow-windows")
    {
        arguments.remove(0);
    }
    arguments
}

/// 防止把旧 `.flow` 或任意文本文件误当作当前 TestFlow 入口。
fn validate_testflow_extension(flow: &Path) -> Result<(), String> {
    if flow.extension().and_then(|extension| extension.to_str()) == Some("testflow") {
        Ok(())
    } else {
        Err("流程文件扩展名必须是 .testflow".into())
    }
}

/// 生成自包含 BAT；流程和 DLL 均相对于 BAT 自身目录定位，可在 UNC 路径启动。
fn batch_body(flow_name: &str, cdylib_name: &str) -> String {
    let payload = embedded_runner();
    let chunks = payload
        .as_bytes()
        .chunks(4000)
        .map(|chunk| {
            format!(
                "echo {}",
                std::str::from_utf8(chunk).expect("Base64 载荷必须是 ASCII")
            )
        })
        .collect::<Vec<_>>();

    format!(
        "@echo off\r\nsetlocal\r\nset \"MIRESIDIAN_FLOW_FILE=%~dp0{flow_name}\"\r\nset \"MIRESIDIAN_DLL_NAME={}.dll\"\r\nset \"MIRESIDIAN_DLL_PATH=%~dp0..\\dist\"\r\nset \"MIRESIDIAN_TESTFLOW_PAYLOAD=%TEMP%\\miresidian-testflow-%RANDOM%-%RANDOM%.b64\"\r\n> \"%MIRESIDIAN_TESTFLOW_PAYLOAD%\" (\r\n{}\r\n)\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -EncodedCommand {}\r\nset \"MIRESIDIAN_TESTFLOW_EXIT=%ERRORLEVEL%\"\r\ndel /q \"%MIRESIDIAN_TESTFLOW_PAYLOAD%\" >nul 2>nul\r\nif not defined MIRESIDIAN_TESTFLOW_NO_PAUSE pause\r\nendlocal & exit /b %MIRESIDIAN_TESTFLOW_EXIT%\r\n",
        cdylib_name.replace('-', "_"),
        chunks.join("\r\n"),
        embedded_bootstrap(),
    )
}

const RUNNER: &str = include_str!("runner.ps1");

/// PowerShell 的 `-EncodedCommand` 要求 UTF-16LE 文本的 Base64 编码。
fn encode_powershell(source: &str) -> String {
    let bytes = source
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    STANDARD.encode(bytes)
}

/// 完整执行器分段写入临时载荷，避免 cmd.exe 的单条命令长度限制。
fn embedded_runner() -> String {
    encode_powershell(RUNNER)
}

/// 短引导命令从临时载荷在内存中恢复并运行完整 TestFlow 执行器。
fn embedded_bootstrap() -> String {
    encode_powershell(
        r#"$payload=[IO.File]::ReadAllText($env:MIRESIDIAN_TESTFLOW_PAYLOAD);$script=[Text.Encoding]::Unicode.GetString([Convert]::FromBase64String($payload));Remove-Item -LiteralPath $env:MIRESIDIAN_TESTFLOW_PAYLOAD -Force;&([ScriptBlock]::Create($script));exit $LASTEXITCODE"#,
    )
}

#[cfg(test)]
mod tests {
    use super::{batch_body, cargo_subcommand_arguments, validate_testflow_extension};
    use std::path::Path;

    #[test]
    fn cargo_子命令名不会被当作流程路径() {
        assert_eq!(
            cargo_subcommand_arguments([
                "miresidian-testflow-windows".to_owned(),
                "tools/smoke.testflow".to_owned(),
            ]),
            vec!["tools/smoke.testflow"]
        );
    }

    #[test]
    fn only_testflow_files_are_accepted() {
        assert!(validate_testflow_extension(Path::new("serial_smoke.testflow")).is_ok());
        assert!(validate_testflow_extension(Path::new("serial_smoke.flow")).is_err());
    }

    #[test]
    fn windows_batch_uses_flow_name_and_cdylib_name() {
        let batch = batch_body("serial_smoke.testflow", "serial-node");
        assert!(batch.contains("MIRESIDIAN_FLOW_FILE=%~dp0serial_smoke.testflow"));
        assert!(batch.contains("MIRESIDIAN_DLL_NAME=serial_node.dll"));
        assert!(batch.contains("MIRESIDIAN_DLL_PATH=%~dp0..\\dist"));
        assert!(!batch.contains("pushd "));
    }

    #[test]
    fn generated_batch_is_self_contained() {
        let batch = batch_body("serial_smoke.testflow", "serial-node");
        assert!(batch.contains("-EncodedCommand"));
        assert!(batch.contains("MIRESIDIAN_TESTFLOW_PAYLOAD"));
        assert!(batch.contains("MIRESIDIAN_TESTFLOW_NO_PAUSE"));
        assert!(batch.contains("endlocal & exit /b %MIRESIDIAN_TESTFLOW_EXIT%"));
        assert!(batch.lines().all(|line| line.len() < 8_191));
        assert!(!batch.contains("invoke_miresidian_dll.ps1"));
        assert!(super::RUNNER.contains("function Invoke-Flow"));
        assert!(super::RUNNER.contains("Get-FlowMethodDefinitions"));
        assert!(super::RUNNER.contains("miresidian_invoke"));
        assert!(super::RUNNER.contains(".ProviderPath"));
        assert!(
            super::RUNNER
                .lines()
                .filter(|line| line.contains("Resolve-Path"))
                .all(|line| line.contains(".ProviderPath"))
        );
    }
}
