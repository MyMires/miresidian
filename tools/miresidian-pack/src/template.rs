//! 最小节点模板：只声明已发布精确依赖，锁文件始终由 Cargo 生成。
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use cargo_metadata::semver::Version;

/// 与独立烟测夹具相同的已发布基线，不随框架源码候选版本联动。
const DEFAULT_FRAMEWORK: &str = "0.14.0";

pub(crate) struct TemplateRequest {
    directory: PathBuf,
    package: String,
    name: String,
    version: String,
}

impl TemplateRequest {
    pub(crate) fn from_args(args: &[String]) -> Result<Self, String> {
        let Some(directory) = args.first().filter(|s| !s.starts_with('-')) else {
            return Err(crate::cli::usage());
        };
        let directory = PathBuf::from(directory);
        let package = directory
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("模板目录末段必须是有效的包名")?
            .to_owned();
        if !package
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            || !package
                .bytes()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
            || syn::parse_str::<syn::Ident>(&package.replace('-', "_")).is_err()
        {
            return Err("模板目录末段须是 Rust 包名（字母、数字、下划线或连字符，不能以数字开头或使用关键字）".into());
        }
        let mut name = None;
        let mut version = None;
        let mut options = args[1..].chunks_exact(2);
        for option in &mut options {
            match option[0].as_str() {
                "--name" if name.is_none() => name = Some(option[1].clone()),
                "--framework-version" if version.is_none() => version = Some(option[1].clone()),
                _ => return Err(crate::cli::usage()),
            }
        }
        if !options.remainder().is_empty() {
            return Err(crate::cli::usage());
        }
        let name = name.unwrap_or_else(|| package.clone());
        crate::validate::validate_node_name(&name)?;
        let version = version.unwrap_or_else(|| DEFAULT_FRAMEWORK.to_owned());
        let parsed = Version::parse(&version)
            .map_err(|_| "--framework-version 须是完整版本号，例如 0.14.0")?;
        if parsed < Version::new(0, 14, 0) {
            return Err("模板要求框架版本至少为 0.14.0，且提供 ABI v7".into());
        }
        Ok(Self {
            directory,
            package,
            name,
            version,
        })
    }
}

pub(crate) fn create(request: TemplateRequest) -> Result<(), String> {
    write_project(&request)?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(&request.directory)
        .args(["generate-lockfile"])
        .status()
        .map_err(|e| format!("无法启动 Cargo：{e}"))?;
    if !status.success() {
        return Err(format!(
            "模板已写入 {}，但锁文件生成失败（{status}）；请确认框架版本已发布及 registry 可访问，模板创建尚未完成",
            request.directory.display()
        ));
    }
    let root = request
        .directory
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let nodes = crate::validate::check_project(&root, None)?;
    if nodes.iter().any(|node| node.abi_version != 7) {
        return Err(
            "模板已写入，但目标框架不是 ABI v7；请按实际头文件调整交付文档，模板创建尚未完成"
                .into(),
        );
    }
    println!(
        "已创建节点模板：{}（框架 ={}）；请运行 cargo check --locked 并补充业务与交付说明",
        root.display(),
        request.version
    );
    Ok(())
}

fn write_project(request: &TemplateRequest) -> Result<(), String> {
    // 原子地取得新目录；即使既有目录为空也不覆盖。后续失败保留文件供检查。
    fs::create_dir(&request.directory).map_err(|e| {
        format!(
            "无法创建模板目录 {}（目录必须不存在，父目录须已存在）：{e}",
            request.directory.display()
        )
    })?;
    for directory in ["src", "docs"] {
        fs::create_dir(request.directory.join(directory)).map_err(|e| e.to_string())?;
    }
    let date = utc_date(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs()
            / 86_400,
    );
    for (file, content) in [
        (
            "Cargo.toml".into(),
            include_str!("../templates/Cargo.toml.in"),
        ),
        ("src/lib.rs".into(), include_str!("../templates/lib.rs.in")),
        (
            "README.md".into(),
            include_str!("../templates/README.md.in"),
        ),
        (
            "CHANGELOG.md".into(),
            include_str!("../templates/CHANGELOG.md.in"),
        ),
        (".gitignore".into(), "/target\n/dist/\n"),
        (
            format!("docs/{}.md", request.name),
            include_str!("../templates/node.md.in"),
        ),
    ] {
        let rendered = render(content, request, &date);
        fs::write(request.directory.join(file), rendered).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 只替换模板本身的占位符，用户名称中的花括号始终保留为原文。
fn render(content: &str, request: &TemplateRequest, date: &str) -> String {
    let literal = format!("{:?}", request.name);
    let values = [
        ("package", request.package.as_str()),
        ("version", request.version.as_str()),
        ("name_literal", literal.as_str()),
        ("name", request.name.as_str()),
        ("date", date),
    ];
    let mut output = String::new();
    let mut remaining = content;
    while let Some(start) = remaining.find("{{") {
        output.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        let Some((token, tail)) = remaining.split_once("}}") else {
            break;
        };
        if let Some((_, value)) = values.iter().find(|(key, _)| *key == &token[2..]) {
            output.push_str(value);
        } else {
            output.push_str(token);
            output.push_str("}}");
        }
        remaining = tail;
    }
    output.push_str(remaining);
    output
}

/// 将 Unix 纪元后的天数转换为 UTC 公历日期，无需依赖平台 date 命令。
fn utc_date(days: u64) -> String {
    let days = days as i64 + 719_468;
    let era = days / 146_097;
    let day_of_era = days % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = month_index + if month_index < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(args: &[&str]) -> Result<TemplateRequest, String> {
        TemplateRequest::from_args(&args.iter().map(|s| (*s).into()).collect::<Vec<_>>())
    }

    #[test]
    fn 模板默认发布基线且拒绝非法名称和版本() {
        let default = request(&["demo-node"]).unwrap();
        assert_eq!(default.version, "0.14.0");
        assert_eq!(default.name, "demo-node");
        for args in [
            vec!["demo", "--framework-version", "^0.14"],
            vec!["demo", "--name", "../escape"],
            vec!["bad.name"],
            vec!["r#demo"],
            vec!["/tmp/-demo"],
            vec!["demo", "--name"],
        ] {
            assert!(request(&args).is_err());
        }
    }

    #[test]
    fn 日期覆盖纪元闰日与当前发布日期() {
        assert_eq!(utc_date(0), "1970-01-01");
        assert_eq!(utc_date(11_016), "2000-02-29");
        assert_eq!(utc_date(20_756), "2026-10-30");
    }

    #[test]
    fn 用户名称中的模板标记保持原文() {
        let request = request(&["demo", "--name", "设备{{date}}"]).unwrap();
        assert_eq!(
            render("{{name}} / {{date}}", &request, "2026-10-30"),
            "设备{{date}} / 2026-10-30"
        );
    }

    #[test]
    fn 模板只写精确发布依赖且不覆盖目录() {
        let temp = crate::test_support::TempDir::new();
        let path = temp.0.join("demo-node");
        let template = request(&[path.to_str().unwrap(), "--name", "演示设备"]).unwrap();
        write_project(&template).unwrap();
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(path.join("Cargo.toml")).unwrap()).unwrap();
        assert_eq!(
            manifest["dependencies"]["miresidian"]["version"].as_str(),
            Some("=0.14.0")
        );
        assert!(manifest["dependencies"]["miresidian"].get("path").is_none());
        assert!(
            !path.join("Cargo.lock").exists(),
            "锁文件须由 Cargo 生成，不能伪造"
        );
        syn::parse_file(&fs::read_to_string(path.join("src/lib.rs")).unwrap()).unwrap();
        assert_eq!(
            crate::source::find_node_name(&path.join("src"))
                .unwrap()
                .as_deref(),
            Some("演示设备")
        );
        assert!(path.join("docs/演示设备.md").is_file());
        fs::write(path.join("README.md"), "用户文件").unwrap();
        assert!(write_project(&template).is_err());
        assert_eq!(
            fs::read_to_string(path.join("README.md")).unwrap(),
            "用户文件"
        );
    }
}
