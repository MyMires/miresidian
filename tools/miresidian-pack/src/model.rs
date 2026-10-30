//! 构建计划、节点与交付产物的内部数据。
use std::path::PathBuf;

/// 一个可发布的 Windows 目标。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WindowsTarget {
    /// Cargo/Rust 使用的目标三元组。
    pub(crate) triple: &'static str,
    /// 发布目录中使用的平台名称。
    pub(crate) package_name: &'static str,
}

/// 用户本次请求的构建范围。
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PackageRequest {
    /// 需要构建的 Windows 目标；未指定架构时包含 x64 与 x86。
    pub(crate) targets: Vec<WindowsTarget>,
    /// 需要构建的节点显示名；为空时构建全部已发现的节点。
    pub(crate) node_name: Option<String>,
}

/// 一个待发布的节点 crate。
pub(crate) struct NodeRelease {
    /// Cargo 实际使用的产物目录，兼容工作区与自定义 target-dir。
    pub(crate) target_directory: PathBuf,
    /// 交给 Cargo 构建的包名称。
    pub(crate) package: String,
    /// 实例项目 Cargo.toml 所在目录，用于收集发布随附文档。
    pub(crate) project_directory: PathBuf,
    /// 由解析到的 miresidian_abi crate 提供的发布头文件。
    pub(crate) abi_header: PathBuf,
    /// 节点的发布目录与命令筛选名称。
    pub(crate) node_name: String,
    /// cdylib 编译完成后的 DLL 文件名。
    pub(crate) dll_name: String,
    /// 实例项目的语义化版本。
    pub(crate) project_version: String,
    /// 从已解析的根框架依赖自动取得的 Miresidian 版本。
    pub(crate) miresidian_version: String,
    /// 从已解析的 ABI 契约自动取得的 ABI 版本。
    pub(crate) abi_version: u32,
}

/// 已复制到发布目录的单个 DLL 及其校验和。
pub(crate) struct ReleaseArtifact {
    pub(crate) platform: &'static str,
    pub(crate) file: String,
    pub(crate) sha256: String,
}
