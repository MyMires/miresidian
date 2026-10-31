//! 节点开发与发布工具入口，具体职责由各模块承担。
mod artifacts;
mod build;
mod cli;
mod discover;
mod framework;
mod manifest;
mod model;
mod package;
mod source;
mod template;
#[cfg(test)]
mod test_support;
mod validate;

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Miresidian 工具失败：{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = cli::cargo_subcommand_arguments(env::args().skip(1));
    match cli::command_from_args(arguments)? {
        cli::Command::Package(request) => package::package(request),
        cli::Command::Build(request) => build::build(request),
        cli::Command::Check(name) => {
            let root = env::current_dir().map_err(|e| format!("无法确定当前项目目录：{e}"))?;
            let nodes = validate::check_project(&root, name.as_deref())?;
            for node in nodes {
                println!(
                    "结构检查通过：{} / {} / 框架 {} / ABI v{}",
                    node.node_name, node.package, node.miresidian_version, node.abi_version
                );
            }
            Ok(())
        }
        cli::Command::New(request) => template::create(request),
        cli::Command::Help => {
            println!("{}", cli::usage());
            Ok(())
        }
        cli::Command::Version => {
            println!("miresidian_pack {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
