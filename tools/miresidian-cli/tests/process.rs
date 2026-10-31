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
    assert!(String::from_utf8_lossy(&output.stderr).contains("安装完整工具套件"));
}
