# 构建环境

项目把常用构建流程收敛为 Cargo 默认成员与别名；日常不需要再手写包名、目标三元组或链接器环境变量。

## 日常开发

在项目根目录执行：

```bash
cargo build
cargo test
cargo lint
```

`cargo build` 默认构建 `constellation/serial` 的串口通信驱动；`cargo lint` 会对整个工作区执行 Clippy 并把警告视为错误。

## 首次准备 Windows DLL 交叉编译环境

此步骤每个容器只需执行一次：

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends build-essential pkg-config mingw-w64
rustup target add x86_64-pc-windows-gnu
rustup target add i686-pc-windows-gnu
```

## 生成测试平台可加载的 Windows x64 DLL

```bash
cargo dll
cargo dll32
```

构建完成后，测试平台可直接部署的产物会被自动整理到：

```text
dist/串口通信/windows-x64/miresidian_serial.dll
dist/串口通信/windows-x86/miresidian_serial.dll
```

`dist/` 及其子目录均由构建命令自动创建，无需预先准备。发布目录的名称取自驱动结构体上的 `#[miresidian::node("串口通信")]`；其中同时包含 C 头文件和同名中文部署说明。`cargo dll` 对应 Windows x64，`cargo dll32` 对应 Windows x86（32 位）。Cargo 配置已记录两个 MinGW 链接器，因此无需再在命令行设置链接器环境变量。如需明确执行所有工作区检查，可使用 `cargo check-all` 与 `cargo test-all`。
