# miresidian_pack

`miresidian_pack` 为 Miresidian 节点项目构建 Windows DLL，并整理为测试平台可交付的发布目录。

## 用法

在节点项目根目录运行：

```text
cargo miresidian-pack
cargo miresidian-pack x64
cargo miresidian-pack x86 串口通信
cargo miresidian-pack x64 串口通信
```

省略参数时会构建所有发现节点的 x64 与 x86 版本。Cargo 必须能找到 `cargo-miresidian-pack` 二进制；在框架源码联调时，可由节点项目的 Cargo alias 指向此工具。

框架发布到 crates.io 后，安装一次即可在任意节点项目使用：

```text
cargo install miresidian_pack
```

## 输入要求

- 节点项目是 Cargo 工作区，且工作区解析到 `miresidian` 和 `miresidian_abi`。
- 节点 crate 声明 `cdylib`，并以 `#[miresidian::node("节点名称")]` 标注节点。
- 节点 `Cargo.toml` 的 `package.metadata.miresidian` 声明框架版本与 ABI 版本。
- Windows 交叉编译所需 target、链接器及 `.cargo/config.toml` 由节点项目自行配置。

## 输出

输出位于 `dist/<节点名称>/`。其中 `windows-x64/` 与 `windows-x86/` 分别包含 DLL 和 `miresidian.h`；根目录包含节点说明、`CHANGELOG.md` 与 `manifest.json`。
`manifest.json` 记录节点、框架、ABI、Git 修订以及每个 DLL 的 SHA-256，供部署端核对完整性。
