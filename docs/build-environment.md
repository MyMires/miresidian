# 构建环境

本仓库仅包含 Miresidian 框架及其 ABI、编解码、过程宏和打包工具。

## 框架开发

在框架根目录执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## 节点项目

节点应作为独立 Cargo 项目维护，并从 crates.io 依赖发布后的框架；本地联调可使用相邻目录的 `path` 依赖。Windows DLL 的交叉编译、`cargo miresidian-pack` 和烟测工具均由节点项目自行配置与维护。
