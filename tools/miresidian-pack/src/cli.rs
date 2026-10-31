//! 命令行参数与构建目标。
use crate::model::{PackageRequest, WindowsTarget};
use crate::template::TemplateRequest;

pub(crate) enum Command {
    Package(PackageRequest),
    Build(PackageRequest),
    Check(Option<String>),
    New(TemplateRequest),
    Help,
    Version,
}

/// 新功能使用长选项，保留原来的节点名称与位数位置参数。
pub(crate) fn command_from_args(arguments: Vec<String>) -> Result<Command, String> {
    if matches!(
        arguments.first().map(String::as_str),
        Some("--new" | "--check" | "--build" | "--pack")
    ) && matches!(arguments.get(1).map(String::as_str), Some("--help" | "-h"))
        && arguments.len() == 2
    {
        return Ok(Command::Help);
    }
    match arguments.first().map(String::as_str) {
        Some("--help" | "-h") if arguments.len() == 1 => Ok(Command::Help),
        Some("--version" | "-V") if arguments.len() == 1 => Ok(Command::Version),
        Some("--check") => Ok(Command::Check(options(&arguments[1..], false)?.node_name)),
        Some("--build") => Ok(Command::Build(options(&arguments[1..], true)?)),
        Some("--pack") => Ok(Command::Package(options(&arguments[1..], true)?)),
        Some("--new") => TemplateRequest::from_args(&arguments[1..]).map(Command::New),
        Some(value) if value.starts_with('-') => Err(usage()),
        _ => request_from_args(arguments).map(Command::Package),
    }
}

/// 长选项入口与旧的位置参数入口分开，避免占用已有节点名称。
fn options(args: &[String], architecture: bool) -> Result<PackageRequest, String> {
    let mut node_name = None;
    let mut arch = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--node" if node_name.is_none() => {
                index += 1;
                node_name = Some(
                    args.get(index)
                        .filter(|s| !s.starts_with('-'))
                        .ok_or_else(usage)?
                        .clone(),
                );
            }
            "--arch" if architecture && arch.is_none() => {
                index += 1;
                arch = Some(args.get(index).ok_or_else(usage)?.as_str());
            }
            value if !architecture && !value.starts_with('-') && node_name.is_none() => {
                node_name = Some(value.to_owned())
            }
            _ => return Err(usage()),
        }
        index += 1;
    }
    let targets = match arch.unwrap_or("all") {
        "all" => vec![windows_x64(), windows_x86()],
        value => vec![target_from_architecture(value)?],
    };
    Ok(PackageRequest { targets, node_name })
}

/// Cargo 调用外部子命令时会额外传入子命令名；直接运行二进制时则不会。
pub(crate) fn cargo_subcommand_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut arguments = arguments.into_iter().collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == "miresidian-pack")
    {
        arguments.remove(0);
    }
    arguments
}

pub(crate) fn request_from_args(
    arguments: impl IntoIterator<Item = String>,
) -> Result<PackageRequest, String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => Ok(PackageRequest {
            targets: vec![windows_x64(), windows_x86()],
            node_name: None,
        }),
        [node_name] if node_name != "x64" && node_name != "x86" => Ok(PackageRequest {
            targets: vec![windows_x64(), windows_x86()],
            node_name: Some(node_name.clone()),
        }),
        [architecture] => Ok(PackageRequest {
            targets: vec![target_from_architecture(architecture)?],
            node_name: None,
        }),
        [architecture, node_name] => Ok(PackageRequest {
            targets: vec![target_from_architecture(architecture)?],
            node_name: Some(node_name.clone()),
        }),
        _ => Err(usage()),
    }
}

fn target_from_architecture(architecture: &str) -> Result<WindowsTarget, String> {
    match architecture {
        "x64" => Ok(windows_x64()),
        "x86" => Ok(windows_x86()),
        _ => Err(usage()),
    }
}

pub(crate) fn usage() -> String {
    "用法：\n  cargo miresidian-pack --new <目录> [--name <节点名称>] [--miresidian-version <版本>|--mires-version <版本>]\n  cargo miresidian-pack --check [--node <节点名称>]\n  cargo miresidian-pack --build [--node <节点名称>] [--arch x64|x86|all]\n  cargo miresidian-pack --pack [--node <节点名称>] [--arch x64|x86|all]\n  cargo miresidian-pack [x86|x64] [节点名称]  （兼容入口）\n  cargo miresidian-pack --help|-h\n  cargo miresidian-pack --version|-V\n默认：最新未撤回正式版框架；全部节点；all 架构；release 构建。\n--framework-version 保留为版本参数的兼容别名。".to_owned()
}

pub(crate) fn windows_x64() -> WindowsTarget {
    WindowsTarget {
        triple: "x86_64-pc-windows-gnu",
        package_name: "windows-x64",
    }
}

pub(crate) fn windows_x86() -> WindowsTarget {
    WindowsTarget {
        triple: "i686-pc-windows-gnu",
        package_name: "windows-x86",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 长选项独立构建且拒绝重复和未知参数() {
        let Command::Build(request) =
            command_from_args(arguments(&["--build", "--arch", "x86", "--node", "设备"])).unwrap()
        else {
            panic!()
        };
        assert_eq!(request.targets, vec![windows_x86()]);
        assert_eq!(request.node_name.as_deref(), Some("设备"));
        for args in [
            vec!["--pack", "--arch", "arm64"],
            vec!["--pack", "--arch", "x64", "--arch", "x86"],
            vec!["--check", "--arch", "all"],
            vec!["--check", "--node"],
            vec!["--check", "--node", "--bad"],
        ] {
            assert!(command_from_args(arguments(&args)).is_err());
        }
        assert!(matches!(
            command_from_args(arguments(&["--new", "--help"])).unwrap(),
            Command::Help
        ));
    }

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn 默认请求构建全部节点的两个位数() {
        let request = request_from_args(arguments(&[])).expect("默认参数应可解析");

        assert_eq!(request.node_name, None);
        assert_eq!(request.targets, vec![windows_x64(), windows_x86()]);
    }

    #[test]
    fn 节点名称默认构建两个位数() {
        let request = request_from_args(arguments(&["串口通信"])).expect("节点名称参数应可解析");

        assert_eq!(request.node_name.as_deref(), Some("串口通信"));
        assert_eq!(request.targets, vec![windows_x64(), windows_x86()]);
    }

    #[test]
    fn cargo_子命令名不会被当作打包参数() {
        let request = request_from_args(cargo_subcommand_arguments(arguments(&[
            "miresidian-pack",
            "x64",
            "串口通信",
        ])))
        .expect("Cargo 子命令参数应可解析");

        assert_eq!(request.node_name.as_deref(), Some("串口通信"));
        assert_eq!(request.targets, vec![windows_x64()]);
    }

    #[test]
    fn 位数与节点名称可以组合() {
        let request =
            request_from_args(arguments(&["x86", "串口通信"])).expect("位数和节点名称参数应可解析");

        assert_eq!(request.node_name.as_deref(), Some("串口通信"));
        assert_eq!(request.targets, vec![windows_x86()]);
    }

    #[test]
    fn 新入口不占用原有节点名称() {
        for name in ["new", "check", "help"] {
            let Command::Package(request) = command_from_args(arguments(&[name])).unwrap() else {
                panic!("原有位置参数仍须表示节点名")
            };
            assert_eq!(request.node_name.as_deref(), Some(name));
        }
        assert!(matches!(
            command_from_args(arguments(&["--check"])).unwrap(),
            Command::Check(None)
        ));
        assert!(command_from_args(arguments(&["--unknown"])).is_err());
        assert!(command_from_args(arguments(&["--check", "a", "b"])).is_err());
    }
}
