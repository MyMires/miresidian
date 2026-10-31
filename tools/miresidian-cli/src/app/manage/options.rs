//! 管理选项与 Cargo 选项分开解析；显式 -- 后的参数由 Cargo 自己处理。
use super::execution::{Action, HELP, tools};
use std::{collections::HashMap, ffi::OsString};

fn version(value: &str) -> Result<String, String> {
    semver::Version::parse(value)
        .map_err(|_| "版本须为完整 X.Y.Z（或显式预发布版本）".to_owned())?;
    Ok(value.to_owned())
}

pub(in crate::app) fn parse(command: &str, args: Vec<OsString>) -> Result<Action, String> {
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
        return Ok(Action::Help);
    }
    let tools = tools()?;
    let mut full = false;
    let mut target = None;
    let mut selected_version = None;
    let mut versions = HashMap::new();
    let mut forwarded = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let raw = &args[index];
        if raw == "--" {
            forwarded.extend_from_slice(&args[index + 1..]);
            break;
        }
        let text = raw.to_str().unwrap_or("");
        let (flag, inline) = text
            .split_once('=')
            .map_or((text, None), |(a, b)| (a, Some(b)));
        match flag {
            "--full" if inline.is_none() && !full && command != "list" && command != "use" => {
                full = true
            }
            "--version" if command != "list" && command != "use" && selected_version.is_none() => {
                let value = take_value(&args, &mut index, inline)?;
                selected_version = Some(version(value.to_str().ok_or("版本必须为 UTF-8")?)?);
            }
            flag if flag.starts_with("--")
                && flag.len() > 10
                && flag.ends_with("-version")
                && command != "list"
                && command != "use" =>
            {
                let alias = &flag[2..flag.len() - 8];
                if !tools.iter().any(|t| t.alias == alias && t.alias != "cli")
                    || versions.contains_key(alias)
                {
                    return Err(format!("未知或重复的工具版本参数：{flag}"));
                }
                let value = take_value(&args, &mut index, inline)?;
                versions.insert(
                    alias.to_owned(),
                    version(value.to_str().ok_or("版本必须为 UTF-8")?)?,
                );
            }
            "--root" | "--jobs" | "-j" | "--target" | "--target-dir" | "--profile"
            | "--features" | "-F" | "--registry" | "--index" | "--git" | "--branch" | "--tag"
            | "--rev" | "--path" | "--bin" | "--example" | "--message-format" | "--color"
            | "--config" | "-Z" => {
                if command == "list" && flag != "--root" {
                    return Err("list 仅接受 --root；更多 Cargo 参数可放在 -- 后".into());
                }
                forwarded.push(raw.clone());
                if inline.is_none() {
                    forwarded.push(take_value(&args, &mut index, None)?);
                }
            }
            "--locked"
            | "--offline"
            | "--frozen"
            | "--force"
            | "-f"
            | "--debug"
            | "--quiet"
            | "-q"
            | "--verbose"
            | "-v"
            | "-vv"
            | "--all-features"
            | "--no-default-features"
            | "--ignore-rust-version"
            | "--no-track"
            | "--keep-going"
            | "--bins"
                if inline.is_none() && command != "list" =>
            {
                forwarded.push(raw.clone())
            }
            _ if !text.starts_with('-') && !text.is_empty() && command != "list" => {
                if target.is_none() {
                    target = Some(text.to_owned());
                } else if command == "use" && selected_version.is_none() {
                    selected_version = Some(version(text)?);
                } else {
                    return Err(format!("多余的管理参数：{text}"));
                }
            }
            _ => return Err(format!("无效管理参数：{}\n{HELP}", raw.to_string_lossy())),
        }
        index += 1;
    }
    if command == "list" {
        let mut result = vec!["install".into(), "--list".into()];
        result.extend(forwarded);
        return Ok(Action::Cargo(result));
    }
    if full && target.is_some() {
        return Err("--full 不能和单个工具同时指定".into());
    }
    if full && selected_version.is_some() {
        return Err("--full 请分别使用 --pack-version/--testflow-version 等工具版本参数".into());
    }
    if !full && !versions.is_empty() {
        return Err("各工具版本参数只能与 --full 配合".into());
    }
    let selected = if full {
        tools
            .iter()
            .filter(|t| t.alias != "cli")
            .collect::<Vec<_>>()
    } else {
        let target = target.ok_or_else(|| format!("请指定工具或 --full\n{HELP}"))?;
        let tool = tools
            .iter()
            .find(|t| t.alias == target || t.package == target)
            .ok_or_else(|| format!("未登记的工具：{target}"))?;
        if tool.alias == "cli" {
            return Err("CLI 本体请使用 cargo install miresidian_cli [--version X.Y.Z]，避免运行中的程序替换自身".into());
        }
        vec![tool]
    };
    if command == "use" && selected_version.is_none() {
        return Err("用法：cargo miresidian use <工具> <X.Y.Z>".into());
    }
    let mut result: Vec<OsString> = vec!["install".into()];
    for tool in selected {
        let version = versions.get(&tool.alias).or(selected_version.as_ref());
        result.push(match version {
            Some(version) => format!("{}@{version}", tool.package).into(),
            None => tool.package.clone().into(),
        });
    }
    result.extend(forwarded);
    Ok(Action::Cargo(result))
}

fn take_value(
    args: &[OsString],
    index: &mut usize,
    inline: Option<&str>,
) -> Result<OsString, String> {
    let value = match inline {
        Some(value) => OsString::from(value),
        None => {
            *index += 1;
            args.get(*index)
                .filter(|v| !v.to_string_lossy().starts_with('-'))
                .ok_or("参数缺少值")?
                .clone()
        }
    };
    if value.is_empty() {
        return Err("参数值不能为空".into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }
    #[test]
    fn 按工具分别选版且透传_cargo_参数() {
        let Action::Cargo(result) = parse(
            "install",
            args(&[
                "--full",
                "--pack-version",
                "0.12.0",
                "--testflow-version=0.13.0",
                "--root",
                "含 空格目录",
                "--locked",
            ]),
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(
            result,
            args(&[
                "install",
                "miresidian_pack@0.12.0",
                "miresidian_testflow_windows@0.13.0",
                "--root",
                "含 空格目录",
                "--locked"
            ])
        );
        let Action::Cargo(result) = parse("update", args(&["--full"])).unwrap() else {
            panic!()
        };
        assert_eq!(
            result,
            args(&["install", "miresidian_pack", "miresidian_testflow_windows"])
        );
        let Action::Cargo(result) = parse("use", args(&["pack", "0.10.0", "--offline"])).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            result,
            args(&["install", "miresidian_pack@0.10.0", "--offline"])
        );
    }
    #[test]
    fn 错误参数在启动_cargo_之前拒绝() {
        for (cmd, values) in [
            ("install", vec!["--full", "pack"]),
            ("install", vec!["--full", "--version", "0.12.0"]),
            (
                "install",
                vec![
                    "--full",
                    "--pack-version",
                    "0.12.0",
                    "--pack-version",
                    "0.12.0",
                ],
            ),
            ("install", vec!["--full", "--bad-version", "0.12.0"]),
            ("use", vec!["pack", "^0.12"]),
            (
                "install",
                vec!["pack", "--version", "0.12.0", "--version", "0.12.0"],
            ),
            ("use", vec!["pack"]),
            ("install", vec!["cli"]),
            ("update", vec![]),
            ("list", vec!["--full"]),
        ] {
            assert!(parse(cmd, args(&values)).is_err(), "{cmd} {values:?}");
        }
    }
}
