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
#[ignore = "需要 crates.io 已发布 0.14.0 及传递依赖"]
fn 初始化当前目录生成真实锁文件并保留其他文件() {
    let project = Project::new();
    let root = project.0.join("demo-node");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("已有文件.txt"), "保留").unwrap();
    success(tool(&root, &["--init", ".", "--mires-version", "0.14.0"]));
    assert!(root.join("Cargo.lock").is_file());
    assert_eq!(
        fs::read_to_string(root.join("已有文件.txt")).unwrap(),
        "保留"
    );
    assert!(
        !tool(&root, &["--init", ".", "--mires-version", "0.14.0"])
            .status
            .success()
    );
}

#[test]
#[ignore = "需要 crates.io 已发布 0.14.0 及传递依赖；可在预热缓存后设置 CARGO_NET_OFFLINE=true"]
fn 生成模板通过真实编译且检查不构建不改锁文件() {
    let project = Project::new();
    success(tool(
        &project.0,
        &[
            "--new",
            "demo-node",
            "--name",
            "演示设备",
            "--mires-version",
            "0.14.0",
        ],
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

/// 多个真实项目共用依赖构建缓存，仍分别使用自身的 registry 锁文件。
fn build_and_invoke(root: &Path, package: &str, abi: u32) {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/pack-registry-tests");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    success(
        Command::new(cargo)
            .current_dir(root)
            .args(["build", "--locked", "--target-dir"])
            .arg(&target)
            .output()
            .unwrap(),
    );
    let library = target.join("debug").join(format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        package.replace('-', "_"),
        std::env::consts::DLL_SUFFIX,
    ));
    let python = std::env::var_os("PYTHON").unwrap_or_else(|| {
        if cfg!(windows) {
            "python".into()
        } else {
            "python3".into()
        }
    });
    success(
        Command::new(python)
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/template_smoke.py"))
            .arg(library)
            .arg(abi.to_string())
            .output()
            .unwrap(),
    );
}

fn assert_guide(root: &Path, abi: u32) {
    let guide = fs::read_to_string(root.join("docs/演示设备.md")).unwrap();
    assert!(guide.contains(&format!("## C ABI v{abi}")));
    assert!(!guide.contains("{{abi_contract}}"));
    assert!(!guide.contains("ABI 尚未解析"));
    if abi == 10 {
        assert!(guide.contains("miresidian_invoke(const char *request_json)"));
        assert!(!guide.contains("const char *function"));
    } else {
        assert!(guide.contains("miresidian_invoke(const char *function"));
    }
}

#[test]
#[ignore = "需要 crates.io 已发布框架及传递依赖；验证 ABI7–10 原生动态库调用"]
fn 已发布框架矩阵生成对应调用说明并真实执行回显() {
    let project = Project::new();
    for (version, abi) in [("0.14.0", 7), ("0.26.1", 8), ("0.27.0", 9), ("0.33.0", 10)] {
        let package = format!("abi-v{abi}-node");
        success(tool(
            &project.0,
            &[
                "--new",
                &package,
                "--name",
                "演示设备",
                "--mires-version",
                version,
            ],
        ));
        let root = project.0.join(&package);
        assert_guide(&root, abi);
        let lock = fs::read(root.join("Cargo.lock")).unwrap();
        success(tool(&root, &["--check"]));
        assert_eq!(fs::read(root.join("Cargo.lock")).unwrap(), lock);
        build_and_invoke(&root, &package, abi);
    }
}

#[test]
#[ignore = "需要联网确认最新正式框架；验证未指定版本的 new/init"]
fn 默认最新版_new_和_init_可完成且文档匹配实际_abi() {
    // 离线运行仍可验证显式版本矩阵，latest 本身按工具契约必须联网。
    if std::env::var("CARGO_NET_OFFLINE").is_ok_and(|value| value == "true" || value == "1") {
        return;
    }
    let project = Project::new();
    for (package, init) in [("latest-new-node", false), ("latest-init-node", true)] {
        let root = project.0.join(package);
        if init {
            fs::create_dir(&root).unwrap();
            fs::write(root.join("保留.txt"), "用户文件").unwrap();
            success(tool(&root, &["--init", ".", "--name", "演示设备"]));
            assert_eq!(
                fs::read_to_string(root.join("保留.txt")).unwrap(),
                "用户文件"
            );
        } else {
            success(tool(&project.0, &["--new", package, "--name", "演示设备"]));
        }
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(root.join("Cargo.toml")).unwrap()).unwrap();
        let version = manifest["dependencies"]["miresidian"]["version"]
            .as_str()
            .unwrap();
        assert!(version.starts_with('='));
        let output = tool(&root, &["--check"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        let abi: u32 = text.trim().rsplit_once("ABI v").unwrap().1.parse().unwrap();
        assert_guide(&root, abi);
        build_and_invoke(&root, package, abi);
        assert!(!tool(&root, &["--init", "."]).status.success());
        assert_guide(&root, abi);
    }
}
