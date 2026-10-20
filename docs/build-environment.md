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

## 生成测试平台可加载的 Windows DLL

```bash
cargo dll
```

`cargo dll` 默认构建工作区中全部节点的 Windows x86 与 x64 DLL。只需要其中一个位数时，分别使用：

```bash
cargo dll32  # 仅生成 Windows x86 DLL
cargo dll64  # 仅生成 Windows x64 DLL
```

三个命令均可在末尾加上节点显示名，只构建指定节点；名称与
`#[miresidian::node("...")]` 的值完全一致：

```bash
cargo dll 串口通信
cargo dll32 串口通信
cargo dll64 串口通信
```

构建完成后，测试平台可直接部署的产物会被自动整理到：

```text
dist/串口通信/windows-x64/miresidian_serial.dll
dist/串口通信/windows-x86/miresidian_serial.dll
dist/串口通信/串口通信.md
dist/串口通信/CHANGELOG.md
dist/串口通信/manifest.json
```

`dist/` 及其子目录均由构建命令自动创建，无需预先准备。发布目录的名称取自驱动结构体上的 `#[miresidian::node("串口通信")]`；每个位数目录包含对应 DLL 和 C 头文件，同名中文部署说明固定放在驱动根目录。Cargo 配置已记录两个 MinGW 链接器，因此无需再在命令行设置链接器环境变量。如需明确执行所有工作区检查，可使用 `cargo check-all` 与 `cargo test-all`。

## 手动验证 C ABI

串口实例的手动脚本为 `tools/test_serial_dll.py`。在脚本顶部设置 `function`、`config`、`input` 后运行：

```powershell
py tools/test_serial_dll.py
```

脚本会将 `result_json` 原样输出；共享的 C ABI 装载与 JSON 编码逻辑位于 `tools/c_abi_shared.py`。
Windows Python 的位数必须与目标 DLL 一致。

纯 Linux 的 `python3` 不能直接加载 Windows PE DLL。要在 Linux 中执行 `dist` 下的实际 DLL，
需要 Wine 与同位数的 Windows Python，并通过 Wine 运行该脚本；若验证物理串口，还需映射 Wine 的 `COMx`。
