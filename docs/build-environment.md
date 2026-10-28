# 构建环境

[文档中心](README.md) · [架构总览](architecture.md)

本仓库仅包含 Miresidian 框架及其 ABI、编解码、过程宏和打包工具。

## 最低工具链

最低 Rust 版本为 **1.89**。框架使用 Rust 2024、let chains，以及 Rust 1.89 稳定的跨平台文件锁 API。
CI 在 Linux 和 Windows 上使用 1.89 执行全工作区、全目标、全特性编译，并运行异常恢复与多进程租约回归。

```bash
cargo +1.89.0 check --workspace --all-targets --all-features --locked
```

## 框架开发

在框架根目录执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

## 节点项目

节点应作为独立 Cargo 项目维护，并从 crates.io 依赖发布后的框架；提交开发源码时可同时声明精确版本与相邻目录的 `path`，使未发布的新版本也能真实锁定、构建和验证；Cargo 发布包仅保留版本约束。Windows DLL 的交叉编译、`cargo miresidian-pack` 和烟测工具均由节点项目自行配置与维护。
