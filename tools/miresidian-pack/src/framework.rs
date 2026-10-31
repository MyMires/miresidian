//! 通过 Cargo 的 registry 解析选择最新正式版，复用 Cargo 的下载、撤回及校验规则。
use cargo_metadata::MetadataCommand;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn latest() -> Result<String, String> {
    if std::env::var("CARGO_NET_OFFLINE").is_ok_and(|v| v == "true" || v == "1") {
        return Err(
            "离线模式无法确认最新框架；请显式传入 --miresidian-version 或 --mires-version".into(),
        );
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("miresidian-latest-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).map_err(|e| e.to_string())?;
    let result = (|| {
        fs::write(root.join("Cargo.toml"), "[package]\nname=\"miresidian_version_probe\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[workspace]\nresolver=\"3\"\n[lib]\npath=\"lib.rs\"\n[dependencies]\nmiresidian=\"*\"\n").map_err(|e| e.to_string())?;
        fs::write(root.join("lib.rs"), "").map_err(|e| e.to_string())?;
        // 不按当前 Rust 版本静默回退旧框架；通配要求排除预发布版，Cargo 排除已撤回版本。
        let metadata = MetadataCommand::new()
            .current_dir(&root)
            .other_options(vec![
                "--config".into(),
                "resolver.incompatible-rust-versions=\"allow\"".into(),
                "--config".into(),
                "net.offline=false".into(),
            ])
            .exec()
            .map_err(|e| format!("查询最新框架失败：{e}"))?;
        let package = metadata
            .packages
            .iter()
            .filter(|p| p.name == "miresidian")
            .filter(|p| p.source.as_ref().is_some_and(|s| s.is_crates_io()))
            .max_by(|a, b| a.version.cmp(&b.version))
            .ok_or("未解析到 crates.io 框架")?;
        if !package.version.pre.is_empty() {
            return Err("最新框架查询意外返回预发布版本".into());
        }
        Ok(package.version.to_string())
    })();
    let _ = fs::remove_dir_all(&root);
    result
}
