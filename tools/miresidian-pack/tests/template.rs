//! 显式运行的 registry 集成测试；默认测试不要求 registry 可访问。
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "miresidian-pack-registry-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tool(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-miresidian-pack"))
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap()
}

fn success(output: Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "需要 crates.io 已发布 0.14.0 及传递依赖；可在预热缓存后设置 CARGO_NET_OFFLINE=true"]
fn 生成模板通过真实编译且检查不构建不改锁文件() {
    let project = Project::new();
    success(tool(
        &project.0,
        &["--new", "demo-node", "--name", "演示设备"],
    ));
    let root = project.0.join("demo-node");
    let lock = fs::read(root.join("Cargo.lock")).unwrap();
    let source = fs::read(root.join("src/lib.rs")).unwrap();
    success(tool(&root, &["--check", "演示设备"]));
    assert_eq!(fs::read(root.join("Cargo.lock")).unwrap(), lock);
    assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), source);
    assert!(!root.join("dist").exists());
    assert!(!root.join("target").exists());

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    success(
        Command::new(cargo)
            .current_dir(&root)
            .args(["check", "--locked"])
            .output()
            .unwrap(),
    );

    let output = tool(&project.0, &["--new", "demo-node"]);
    assert!(!output.status.success());
    assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), source);

    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(&manifest_path, manifest.replace("\"cdylib\"", "\"rlib\"")).unwrap();
    let output = tool(&root, &["--check"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cdylib"));
    fs::write(&manifest_path, &manifest).unwrap();

    fs::write(&manifest_path, manifest.replace("=0.14.0", "0.14.0")).unwrap();
    let output = tool(&root, &["--check"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("精确版本"));
    fs::write(&manifest_path, manifest).unwrap();

    fs::remove_file(root.join("docs/演示设备.md")).unwrap();
    let output = tool(&root, &["--check"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("交付文档"));
}
