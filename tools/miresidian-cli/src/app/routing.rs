//! 命令和独立程序的映射由套件清单维护，脚本发布也读取同一份清单。
use std::ffi::OsString;
use toml::Value;

pub(super) const TOOLSET: &str = include_str!("../../toolset.toml");
pub(super) const HELP: &str = "用法：cargo miresidian <命令>（cargo mires 为等价入口）\n  new <目录> [--name <名称>] [--miresidian-version <版本>|--mires-version <版本>]\n  check [--node <名称>]\n  build [--node <名称>] [--arch x64|x86|all]\n  pack [--node <名称>] [--arch x64|x86|all]\n  testflow --os windows <流程.testflow>\n  <命令> --help\n  --help|-h\n  --version|-V\nbuild/pack 默认全部节点、all 架构、release；new 默认最新正式版框架。";

pub(super) enum Action {
    Help,
    TestflowHelp,
    Version,
    Invoke {
        binary: String,
        package: String,
        minimum: String,
        arguments: Vec<OsString>,
        check_flow: bool,
    },
}

pub(super) fn parse(mut args: Vec<OsString>) -> Result<Action, String> {
    if args
        .first()
        .is_some_and(|s| s == "miresidian" || s == "mires")
    {
        args.remove(0);
    }
    if args.is_empty() || matches!(args[0].to_str(), Some("--help" | "-h")) && args.len() == 1 {
        return Ok(Action::Help);
    }
    if matches!(args[0].to_str(), Some("--version" | "-V")) && args.len() == 1 {
        return Ok(Action::Version);
    }
    let command = args.remove(0);
    let command = command.to_str().ok_or("命令名必须为 UTF-8")?;
    let mut os = None;
    if command == "testflow" {
        if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
            return Ok(Action::TestflowHelp);
        }
        let mut remaining = Vec::new();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            if arg == "--os" {
                if os.is_some() {
                    return Err("--os 不能重复".into());
                }
                os = Some(iter.next().ok_or("--os 缺少值")?);
            } else if arg.to_string_lossy().starts_with('-') {
                return Err("testflow 仅接受 --os 和一个流程路径".into());
            } else {
                remaining.push(arg);
            }
        }
        if os.is_none() || remaining.len() != 1 {
            return Err("用法：cargo miresidian testflow --os windows <流程.testflow>".into());
        }
        args = remaining;
    }
    let manifest: Value = toml::from_str(TOOLSET).map_err(|e| format!("工具清单无效：{e}"))?;
    let route = manifest["routes"]
        .as_array()
        .ok_or("缺少路由")?
        .iter()
        .find(|r| {
            r["command"].as_str() == Some(command)
                && r.get("os").and_then(Value::as_str) == os.as_ref().and_then(|s| s.to_str())
        })
        .ok_or_else(|| format!("未知命令或未支持的系统：{command}\n{HELP}"))?;
    let package = route["tool"].as_str().ok_or("路由缺少工具")?;
    let tool = manifest["tools"]
        .as_array()
        .ok_or("缺少工具")?
        .iter()
        .find(|t| t["package"].as_str() == Some(package))
        .ok_or("未登记后端")?;
    if let Some(prefix) = route["prefix"].as_str().filter(|s| !s.is_empty()) {
        args.insert(0, prefix.into());
    }
    let check_flow = command == "check" && !args.iter().any(|s| s == "--help" || s == "-h");
    Ok(Action::Invoke {
        binary: tool["binaries"][0].as_str().ok_or("缺少程序名")?.to_owned(),
        minimum: tool["version"].as_str().ok_or("缺少版本")?.to_owned(),
        package: package.to_owned(),
        arguments: args,
        check_flow,
    })
}

pub(super) fn backend(package: &str) -> Result<(String, String), String> {
    let manifest: Value = toml::from_str(TOOLSET).map_err(|e| e.to_string())?;
    let tool = manifest["tools"]
        .as_array()
        .ok_or("缺少工具")?
        .iter()
        .find(|t| t["package"].as_str() == Some(package))
        .ok_or("未登记后端")?;
    Ok((
        tool["binaries"][0].as_str().ok_or("缺少程序名")?.to_owned(),
        tool["version"].as_str().ok_or("缺少版本")?.to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }
    #[test]
    fn 两入口参数保持一致且操作系统只用于选择工具() {
        for name in ["mires", "miresidian"] {
            let Action::Invoke {
                binary, arguments, ..
            } = parse(args(&[
                name,
                "pack",
                "--node",
                "含 空格设备",
                "--arch",
                "x86",
            ]))
            .unwrap()
            else {
                panic!()
            };
            assert_eq!(binary, "cargo-miresidian-pack");
            assert_eq!(
                arguments,
                args(&["--pack", "--node", "含 空格设备", "--arch", "x86"])
            );
        }
        let Action::Invoke { arguments, .. } =
            parse(args(&["testflow", "tools/a b.testflow", "--os", "windows"])).unwrap()
        else {
            panic!()
        };
        assert_eq!(arguments, args(&["tools/a b.testflow"]));
        for values in [
            vec!["testflow", "x"],
            vec!["testflow", "--os", "linux", "x"],
            vec!["testflow", "--os", "windows", "--os", "windows", "x"],
        ] {
            assert!(parse(args(&values)).is_err());
        }
    }
}
