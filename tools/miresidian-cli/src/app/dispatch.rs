//! 后端以独立进程执行，继承当前目录和标准流，不通过 shell 拼接参数。
use super::routing::{self, Action};
use std::{env, ffi::OsString, io, path::PathBuf, process::Command};

pub(crate) fn run() -> i32 {
    match execute() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Miresidian CLI：{error}");
            1
        }
    }
}

fn executable(binary: &str) -> Result<PathBuf, String> {
    let filename = format!("{binary}{}", env::consts::EXE_SUFFIX);
    let sibling = env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name(&filename);
    if sibling.is_file() {
        return Ok(sibling);
    }
    // 返回程序名交给操作系统搜索 PATH；不存在时统一输出安装提示。
    Ok(filename.into())
}

fn invoke(binary: &str, package: &str, minimum: &str, args: &[OsString]) -> Result<i32, String> {
    let path = executable(binary)?;
    let missing = || {
        format!(
            "找不到 {binary}；请安装完整工具套件，或执行 cargo install {package} --version {minimum} --locked"
        )
    };
    let version = Command::new(&path).arg("--version").output().map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            missing()
        } else {
            format!("无法启动 {binary}：{e}")
        }
    })?;
    let text = String::from_utf8_lossy(&version.stdout);
    let parsed = text
        .split_whitespace()
        .last()
        .and_then(|s| semver::Version::parse(s).ok());
    let minimum = semver::Version::parse(minimum).map_err(|e| e.to_string())?;
    if !version.status.success() || parsed.is_none_or(|v| v < minimum) {
        return Err(format!(
            "{binary} 版本过旧或无法识别；需要至少 {minimum}，请升级独立工具或重新安装套件"
        ));
    }
    let status = Command::new(path)
        .args(args)
        .status()
        .map_err(|e| format!("调用 {binary} 失败：{e}"))?;
    Ok(status.code().unwrap_or(1))
}

fn execute() -> Result<i32, String> {
    match routing::parse(env::args_os().skip(1).collect())? {
        Action::Help => {
            println!("{}", routing::HELP);
            Ok(0)
        }
        Action::TestflowHelp => {
            println!(
                "用法：cargo miresidian testflow --os windows <流程.testflow>\n--os 必填，第一版仅支持 windows；可放在流程路径前后。\n检查流程语法并生成同名 .bat；不构建 DLL、不执行流程。\n请先 pack，然后在 Windows 运行 BAT；自动化可设置 MIRESIDIAN_TESTFLOW_NO_PAUSE=1。"
            );
            Ok(0)
        }
        Action::Version => {
            println!("miresidian_cli {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        Action::Invoke {
            binary,
            package,
            minimum,
            arguments,
            check_flow,
        } => {
            let code = invoke(&binary, &package, &minimum, &arguments)?;
            // 原 pack 检查仍可独立运行；统一 check 额外检查约定的烟测文件。
            let flow = PathBuf::from("tools/smoke.testflow");
            if code == 0 && check_flow && flow.is_file() {
                let package = "miresidian_testflow_windows";
                let (binary, minimum) = routing::backend(package)?;
                return invoke(
                    &binary,
                    package,
                    &minimum,
                    &["--check".into(), flow.into_os_string()],
                );
            }
            Ok(code)
        }
    }
}
