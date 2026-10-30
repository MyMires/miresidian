//! 依赖声明与 Cargo 实际解析来源必须同时满足发布规则。
use std::{collections::HashSet, env, fs, path::Path};

use cargo_metadata::{Metadata, Package, semver::Version};
use toml::Value;

const FRAMEWORK: &[&str] = &[
    "miresidian",
    "miresidian_abi",
    "miresidian_codec",
    "miresidian_macro",
    "miresidian_instances",
    "miresidian_lease",
];
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

pub(crate) fn validate_resolved_sources(packages: &[&Package]) -> Result<(), String> {
    for package in packages {
        if FRAMEWORK.contains(&package.name.as_str())
            && !package.source.as_ref().is_some_and(|s| s.is_crates_io())
        {
            return Err(format!(
                "{} 必须来自 crates.io 已发布版本，禁止 path/git/本地覆盖",
                package.name
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_manifests(metadata: &Metadata, invocation: &Path) -> Result<(), String> {
    let mut files = HashSet::new();
    let workspace = metadata.workspace_root.as_std_path();
    files.insert(workspace.join("Cargo.toml"));
    for package in &metadata.packages {
        if metadata.workspace_members.contains(&package.id) {
            files.insert(package.manifest_path.as_std_path().to_owned());
        }
    }
    // Cargo 会读取调用目录的祖先配置及 CARGO_HOME；不能仅检查项目自己的配置。
    for ancestor in invocation.ancestors().chain(workspace.ancestors()) {
        files.insert(ancestor.join(".cargo/config"));
        files.insert(ancestor.join(".cargo/config.toml"));
    }
    let cargo_home = env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cargo")))
        .or_else(|| env::var_os("USERPROFILE").map(|h| std::path::PathBuf::from(h).join(".cargo")));
    if let Some(directory) = cargo_home {
        files.insert(directory.join("config"));
        files.insert(directory.join("config.toml"));
    }
    for path in files {
        if path.is_file() {
            validate_document(&read_toml(&path)?)
                .map_err(|e| format!("{}：{e}", path.display()))?;
        }
    }
    validate_lock(&read_toml(&workspace.join("Cargo.lock"))?)
}

fn read_toml(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("无法读取 {}：{e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("无法解析 {}：{e}", path.display()))
}

/// 接受继承的 workspace 依赖，但其源声明仍会在工作区清单中检查。
fn validate_document(value: &Value) -> Result<(), String> {
    let Some(table) = value.as_table() else {
        return Ok(());
    };
    if table
        .get("paths")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty())
    {
        return Err("禁止 Cargo paths 本地覆盖".into());
    }
    for (key, child) in table {
        if matches!(
            key.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            if let Some(dependencies) = child.as_table() {
                for (name, declaration) in dependencies {
                    let package = declaration
                        .get("package")
                        .and_then(Value::as_str)
                        .unwrap_or(name);
                    if FRAMEWORK.contains(&package) {
                        validate_declaration(package, declaration)?;
                    }
                }
            }
        } else if key == "patch" || key == "replace" {
            reject_framework_override(child)?;
        } else if key != "metadata" {
            validate_document(child)?;
        }
    }
    Ok(())
}

fn validate_declaration(name: &str, declaration: &Value) -> Result<(), String> {
    if declaration.get("workspace").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    if declaration.get("path").is_some()
        || declaration.get("git").is_some()
        || declaration
            .get("registry")
            .and_then(Value::as_str)
            .is_some_and(|s| s != "crates-io")
    {
        return Err(format!(
            "{name} 必须依赖 crates.io，禁止 path/git/其他 registry"
        ));
    }
    let version = declaration
        .as_str()
        .or_else(|| declaration.get("version").and_then(Value::as_str));
    if !version
        .and_then(|v| v.strip_prefix('='))
        .is_some_and(|v| Version::parse(v).is_ok())
    {
        return Err(format!("{name} 必须声明精确版本，例如 =0.14.0"));
    }
    Ok(())
}

fn reject_framework_override(value: &Value) -> Result<(), String> {
    if let Some(table) = value.as_table() {
        for (name, entry) in table {
            let package = entry
                .get("package")
                .and_then(Value::as_str)
                .unwrap_or_else(|| name.split(':').next().unwrap_or(name));
            if FRAMEWORK.contains(&package) {
                return Err(format!("禁止 patch/replace 覆盖框架依赖 {package}"));
            }
            reject_framework_override(entry)?;
        }
    }
    Ok(())
}

fn validate_lock(lock: &Value) -> Result<(), String> {
    let packages = lock
        .get("package")
        .and_then(Value::as_array)
        .ok_or("Cargo.lock 缺少 package 列表")?;
    for package in packages {
        let name = package.get("name").and_then(Value::as_str).unwrap_or("");
        if !FRAMEWORK.contains(&name) {
            continue;
        }
        let checksum = package
            .get("checksum")
            .and_then(Value::as_str)
            .unwrap_or("");
        if package.get("source").and_then(Value::as_str) != Some(CRATES_IO)
            || checksum.len() != 64
            || !checksum.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(format!(
                "Cargo.lock 中 {name} 缺少 crates.io 来源或有效校验和；请由 Cargo 重新生成"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 拒绝范围依赖及别名覆盖() {
        for declaration in [
            "miresidian = \"0.14\"",
            "core = { package = \"miresidian\", version = \"=0.14.0\", path = \"../local\" }",
            "miresidian = { git = \"https://example.invalid/core\" }",
        ] {
            let document: Value =
                toml::from_str(&format!("[dependencies]\n{declaration}")).unwrap();
            assert!(validate_document(&document).is_err());
        }
        let document: Value = toml::from_str("[workspace.dependencies]\nmiresidian = \"=0.14.0\"\n[dependencies]\nmiresidian.workspace = true").unwrap();
        validate_document(&document).unwrap();
    }

    #[test]
    fn 拒绝框架覆盖但允许其他库本地依赖() {
        for text in [
            "[patch.crates-io]\nmiresidian = { path = '../local' }",
            "[replace]\n'miresidian:0.14.0' = { path = '../local' }",
            "paths = ['../local']",
        ] {
            assert!(validate_document(&toml::from_str(text).unwrap()).is_err());
        }
        validate_document(
            &toml::from_str("[dependencies]\nhelper = { path = '../helper' }").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn 锁文件不能伪装本地框架为发布版本() {
        let text = "[[package]]\nname = 'miresidian'\nversion = '0.14.0'\n";
        assert!(validate_lock(&toml::from_str(text).unwrap()).is_err());
        let valid = format!(
            "{text}source = '{CRATES_IO}'\nchecksum = '{}'",
            "a".repeat(64)
        );
        validate_lock(&toml::from_str(&valid).unwrap()).unwrap();
    }
}
