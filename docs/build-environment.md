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

`tools/common/invoke_miresidian_dll.ps1` 是首选的通用调用工具。Windows 自带的 PowerShell/.NET 即可运行，因此将 `tools` 文件夹复制到部署机后，不需要安装 Python、pip 或第三方包；DLL 也不必保留项目中的目录结构。传入 DLL 路径即可调用，省略操作名时默认调用 `GetFunctions`：

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\common\invoke_miresidian_dll.ps1 .\miresidian_serial.dll
powershell -ExecutionPolicy Bypass -File .\tools\common\invoke_miresidian_dll.ps1 .\miresidian_serial.dll InitDevice '["COM3","9600","8","None","1","None","1000","1000"]' '[]'
powershell -ExecutionPolicy Bypass -File .\tools\common\invoke_miresidian_dll.ps1 .\miresidian_serial.dll -AbiVersion
```

脚本默认原样输出 `result_json`；加 `-Pretty` 会反序列化响应，并将状态、消息和每条业务输出整理为易读的编号清单。PowerShell 进程位数必须与目标 DLL 一致：64 位 DLL 用 Windows x64 PowerShell，32 位 DLL 用 `%WINDIR%\SysWOW64\WindowsPowerShell\v1.0\powershell.exe`。

不想使用命令行时，可双击 `tools/miresidian-serial/test_serial_dll.bat`。在本项目中，它默认从 `dist` 目录自动选择与 PowerShell 位数相同的 DLL，并运行 BAT 顶部 `FLOW_FILE` 指向的 UTF-8 流程文件。流程以 `Config("...")`、`InitDevice()`、`Delay("500")`、`业务方法("...")`、`CloseDevice()` 的形式顺序调用；`Config` 显式指定 C ABI Config，`Delay("毫秒数")` 可在步骤间等待，真实 DLL 方法的参数进入 Input；执行器从 `GetFunctions()` 读取并验证方法存在性，在同一个 DLL 进程中保留初始化状态。DLL 定位为节点脚本内部固定配置。BAT 支持从网络共享（UNC 路径）启动，并会在结束时暂停，方便查看调用结果或错误信息。

`common/invoke_miresidian_dll.ps1` 是通用的 Miresidian C ABI 调用器，并不包含串口业务逻辑。新增节点测试时复制整个 DLL 项目测试目录（例如 `tools/miresidian-serial/`），使 BAT 与 `.flow` 保持同级；修改 BAT 的 `FLOW_FILE`、固定 DLL 文件名及流程中的真实 DLL 方法。所有项目共用 `tools/common/` 内的 PowerShell 调用器，不需要复制或修改。

BAT 中字符串数组的 JSON 写法、转义规则、返回输出格式及新增节点入口的约定见 [DLL 测试工具说明](../tools/README.md)。

纯 Linux 的 `python3` 不能直接加载 Windows PE DLL。要在 Linux 中执行 `dist` 下的实际 DLL，
需要 Wine 与同位数的 Windows Python，并通过 Wine 运行该脚本；若验证物理串口，还需映射 Wine 的 `COMx`。
