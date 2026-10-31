//! 真实进程测试：两种 Cargo 入口、含空格参数/目录、同目录优先和退出码。
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("mires cli {} {stamp}", std::process::id()));
        fs::create_dir(&root).unwrap();
        for (name, source) in [
            ("cargo-miresidian", env!("CARGO_BIN_EXE_cargo-miresidian")),
            ("cargo-mires", env!("CARGO_BIN_EXE_cargo-mires")),
        ] {
            fs::copy(
                source,
                root.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
            )
            .unwrap();
        }
        Self(root)
    }
    fn backend(&self, version: &str) {
        let source = self.0.join("backend.rs");
        fs::write(&source, format!(r#"
use std::{{env,fs}};
fn main() {{
 let args:Vec<String>=env::args().skip(1).collect();
 if args==["--version"] {{ println!("miresidian_pack {version}"); return; }}
 fs::write(env::var_os("MIRES_TEST_LOG").unwrap(),format!("{{:?}}\n{{:?}}",env::current_dir().unwrap(),args)).unwrap();
 std::process::exit(37);
}}
"#)).unwrap();
        assert!(
            Command::new("rustc")
                .arg(&source)
                .arg("--crate-name")
                .arg("test_backend")
                .arg("-o")
                .arg(self.0.join(format!(
                    "cargo-miresidian-pack{}",
                    std::env::consts::EXE_SUFFIX
                )))
                .status()
                .unwrap()
                .success()
        );
    }
    fn command(&self, name: &str) -> Command {
        let mut command = Command::new(
            self.0
                .join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        );
        command
            .current_dir(&self.0)
            .env("MIRES_TEST_LOG", self.0.join("log"));
        command
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn 独立进程保持参数目录和退出码() {
    let fixture = Fixture::new();
    fixture.backend("0.12.0");
    for name in ["cargo-miresidian", "cargo-mires"] {
        let output = fixture
            .command(name)
            .args(["pack", "--node", "设备 有空格", "--arch", "x86"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(37));
        let log = fs::read_to_string(fixture.0.join("log")).unwrap();
        assert!(log.contains("设备 有空格"));
        assert!(log.contains("--pack"));
        assert!(log.contains(&format!("{:?}", fixture.0)));
    }
    fixture.backend("0.10.0");
    let output = fixture
        .command("cargo-mires")
        .arg("check")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("版本过旧"));
}

#[test]
fn 缺少工具提示安装而不自动下载() {
    let fixture = Fixture::new();
    let output = fixture
        .command("cargo-mires")
        .arg("check")
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("install --full"));
}

#[test]
fn 管理命令委托_cargo_并透传退出码() {
    let fixture = Fixture::new();
    let source = fixture.0.join("cargo_stub.rs");
    fs::write(
        &source,
        r#"
use std::{env,fs};
fn main() {
 let args:Vec<String>=env::args().skip(1).collect();
 fs::write(env::var_os("MIRES_TEST_LOG").unwrap(),format!("{:?}",args)).unwrap();
 println!("cargo-stdout");
 eprintln!("cargo-stderr");
 std::process::exit(env::var("MIRES_TEST_EXIT").unwrap().parse().unwrap());
}


"#,
    )
    .unwrap();
    let cargo = fixture
        .0
        .join(format!("cargo-stub{}", std::env::consts::EXE_SUFFIX));
    assert!(
        Command::new("rustc")
            .arg(&source)
            .args(["--crate-name", "cargo_stub", "-o"])
            .arg(&cargo)
            .status()
            .unwrap()
            .success()
    );
    for entry in ["cargo-miresidian", "cargo-mires"] {
        let output = fixture
            .command(entry)
            .args([
                "install",
                "--full",
                "--pack-version",
                "0.12.0",
                "--root",
                "含 空格目录",
                "--locked",
            ])
            .env("CARGO", &cargo)
            .env("MIRES_TEST_EXIT", "37")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(37));
        assert!(String::from_utf8_lossy(&output.stdout).contains("cargo-stdout"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("cargo-stderr"));
        let log = fs::read_to_string(fixture.0.join("log")).unwrap();
        assert!(log.contains("miresidian_pack@0.12.0"));
        assert!(log.contains("miresidian_testflow_windows"));
        assert!(log.contains("含 空格目录"));
        assert!(!log.contains("miresidian_cli"));
    }
    for (args, expected) in [
        (vec!["update", "pack"], "miresidian_pack"),
        (vec!["use", "pack", "0.10.0"], "miresidian_pack@0.10.0"),
        (vec!["list"], "--list"),
    ] {
        let output = fixture
            .command("cargo-mires")
            .args(args)
            .env("CARGO", &cargo)
            .env("MIRES_TEST_EXIT", "0")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(
            fs::read_to_string(fixture.0.join("log"))
                .unwrap()
                .contains(expected)
        );
    }
    fs::remove_file(fixture.0.join("log")).unwrap();
    let output = fixture
        .command("cargo-mires")
        .args(["install", "--full", "--version", "0.12.0"])
        .env("CARGO", &cargo)
        .env("MIRES_TEST_EXIT", "0")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!fixture.0.join("log").exists());
}

#[test]
fn 原生_cargo_在同一目录升级降级并保留安装记录() {
    let fixture = Fixture::new();
    let source = fixture.0.join("local-tool");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("main.rs"),
        "fn main(){println!(\"miresidian_pack {}\",env!(\"CARGO_PKG_VERSION\"));}",
    )
    .unwrap();
    let install_root = fixture.0.join("cargo root");
    let backend = install_root.join("bin").join(format!(
        "cargo-miresidian-pack{}",
        std::env::consts::EXE_SUFFIX
    ));
    let write_manifest = |version: &str| {
        fs::write(source.join("Cargo.toml"),format!("[package]\nname=\"miresidian_pack\"\nversion=\"{version}\"\nedition=\"2021\"\n[workspace]\n[[bin]]\nname=\"cargo-miresidian-pack\"\npath=\"main.rs\"\n")).unwrap()
    };
    let run = |args: &[&str]| {
        let output = fixture
            .command("cargo-mires")
            .args(args)
            .args(["--path"])
            .arg(&source)
            .args(["--root"])
            .arg(&install_root)
            .arg("--offline")
            .env("CARGO", env!("CARGO"))
            .env("CARGO_TARGET_DIR", fixture.0.join("tool-target"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    write_manifest("0.12.0");
    run(&["install", "pack"]);
    assert!(install_root.join(".crates.toml").is_file());
    write_manifest("0.11.0");
    run(&["use", "pack", "0.11.0"]);
    let version = Command::new(&backend).arg("--version").output().unwrap();
    assert!(String::from_utf8_lossy(&version.stdout).contains("0.11.0"));
    write_manifest("0.12.0");
    run(&["update", "pack"]);
    let version = Command::new(&backend).arg("--version").output().unwrap();
    assert!(String::from_utf8_lossy(&version.stdout).contains("0.12.0"));
    let list = fixture
        .command("cargo-mires")
        .args(["list", "--root"])
        .arg(&install_root)
        .env("CARGO", env!("CARGO"))
        .output()
        .unwrap();
    assert!(list.status.success());
    let list = String::from_utf8_lossy(&list.stdout);
    assert!(list.contains("miresidian_pack v0.12.0"));
    assert!(!list.contains("miresidian_pack v0.11.0"));
    assert_eq!(fs::read_dir(install_root.join("bin")).unwrap().count(), 1);
}
