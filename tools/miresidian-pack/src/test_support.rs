//! 纯文件交付测试夹具，不构造设备或覆盖框架依赖。
use crate::model::NodeRelease;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

pub(crate) struct TempDir(pub(crate) PathBuf);

impl TempDir {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "miresidian-pack-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    pub(crate) fn node(&self) -> NodeRelease {
        let docs = self.0.join("docs");
        fs::create_dir(&docs).unwrap();
        fs::write(docs.join("测试节点.md"), "设备调用说明").unwrap();
        fs::write(self.0.join("CHANGELOG.md"), "修订说明").unwrap();
        fs::write(self.0.join("miresidian.h"), "ABI header").unwrap();
        NodeRelease {
            package: "test-node".into(),
            project_directory: self.0.clone(),
            target_directory: self.0.join("custom-target"),
            abi_header: self.0.join("miresidian.h"),
            node_name: "测试节点".into(),
            dll_name: "test_node.dll".into(),
            project_version: "0.1.0".into(),
            miresidian_version: "0.14.0".into(),
            abi_version: 7,
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
