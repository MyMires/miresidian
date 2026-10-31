# 工具操作参考

仅在 Node 项目工作需要相应操作时查阅。命令基线为 CLI 0.2.1、Pack 0.12.0、Windows TestFlow 0.12.0；框架版本独立。实际版本不同时先看帮助，不猜测新参数或例行升级。

## 创建和交付常用操作

统一入口 cargo mires 与 cargo miresidian 等价。在目标节点项目中执行检查、构建、打包和流程生成；创建时从目标目录的父目录执行。

| 需要的操作 | 命令 |
| --- | --- |
| 创建项目 | `cargo miresidian new demo-node --name "演示设备"` |
| 创建时指定框架 | `cargo miresidian new demo-node --miresidian-version X.Y.Z`，或等价的 `--mires-version X.Y.Z` |
| 编译类型检查 | `cargo check --locked` |
| 结构/依赖/说明检查 | `cargo miresidian check` |
| 只编译 x64 DLL | `cargo miresidian build --arch x64` |
| 构建并整理 x64 发布包 | `cargo miresidian pack --arch x64` |
| 生成 Windows 烟测 BAT | `cargo miresidian testflow --os windows tools/smoke.testflow` |
| 单独静态检查流程 | `cargo miresidian-testflow-windows --check tools/smoke.testflow` |

new 的目标目录须不存在、父目录须存在；包名取目录末段，显示名须适合作为 Windows 文件名。未指定框架版本时查询最新未撤回正式版并精确锁定；已有项目不因此自动升级。创建会生成真实锁文件但不代替编译，失败后保留的文件不等于创建成功。

build/pack 默认全部发现节点和 all 架构；用 --node "名称" 选择节点，--arch x64/x86/all 选择本次位数。均 release/locked 构建；pack 已包含编译，输出 dist/<节点显示名>/。清单的 artifacts 反映本次构建，旧位数目录可能仍存在。

check 不编译、不加载 DLL、不生成 dist；当前目录有 tools/smoke.testflow 时还静态检查该流程。它不能证明业务实现、文档准确或设备正常。

## TestFlow 写法与执行

流程使用真实操作名。参数均为 JSON 字符串，包括数值；初始化前显式 Config，业务完成后关闭：

```text
GetFunctions()
GetInitConfig()
Config("设备地址", "1000")
InitDevice()
实际操作名("业务输入")
CloseDevice()
CloseAllDevices()
```

上例是写法示意，必须替换为节点实际配置与操作；无业务配置时用 Config()。延时为 Delay("100")，单位毫秒，范围 0..2147483647；同一配置的静态关联按原始字符串数组判断。

生成命令不构建 DLL。在 Windows 用匹配架构的 DLL/PowerShell 加载环境执行 tools\\smoke.bat；自动化设置 MIRESIDIAN_TESTFLOW_NO_PAUSE=1，核对退出码和输出。BAT 相对自身目录查找流程与 ../dist，保持相对结构。不存在 generate 子命令或 --platform 参数。

## Feature 配置的检查与构建

能力配置与节点声明见 [框架能力与 features](features.md)。通常在依赖声明中固定实际需要的 features，再运行普通检查、测试和打包，使交付使用同一配置。

```text
# 检查当前节点实际解析到的框架 feature（在节点项目运行）。
cargo tree --locked -e features -i miresidian
# 临时启用依赖 feature，只用于相应诊断/检查，不修改 Cargo.toml。
cargo check --locked --features miresidian/instances
# 节点确有 multi-device 转发 feature 时才可使用。
cargo check --locked --features multi-device
cargo test --locked --features multi-device
```

不能在未定义同名项目 feature 的节点中直接写 --features instances，来假设会开启框架依赖。--all-features 也不是独立开启依赖全部 features 的通用替代方式。

CLI/Pack 当前 build、pack 没有 --features 或 --no-default-features 选项；不要构造 cargo miresidian pack --features ...。Pack 调用普通 release/locked 构建，节点交付需要的能力应在依赖 features 或项目默认 features 中有效启用。特殊非默认组合可由原生 Cargo 单独检查/构建，但不能声称 Pack 已支持该组合参数。

## 仅在缺少工具或确需调整版本时

```text
cargo install miresidian_cli
cargo miresidian install --full
```

默认安装到 Cargo 用户级根目录的 bin 并由 Cargo 管理，不建项目内工具目录，不依赖 Python 用户安装器。原生 cargo install 不接受自定义 --full。

查看已安装版本用 cargo miresidian list；更新后端用 update --full；固定后端用 install --full --pack-version 0.12.0 --testflow-version 0.12.0；切换单个版本用 use pack 0.12.0 或 use testflow 0.12.0。这些均为 cargo miresidian 的子命令。CLI 本体用 cargo install miresidian_cli [--version X.Y.Z]。同一根目录替换当前版本，批量失败可能部分成功，先核对结果，不自动反复安装。

## 仅处理本次目标的环境缺项

install --full 不自动配齐构建环境。Pack 0.12.0 目标为 x86_64-pc-windows-gnu 和 i686-pc-windows-gnu，对应 x64/x86。复用现有 Rust、目标标准库和 MinGW 链接器；只处理本次需要的缺项。rustup target add 不安装系统链接器，MSVC-only 配置不能直接视为 Pack 的 GNU 配置。

模板基线 Rust 1.89，实际以所选框架及业务依赖要求为准。环境缺失时说明具体缺项和受阻阶段，不把节点开发扩成全平台自动配置项目。复杂工具参数查看已安装版本 --help。
