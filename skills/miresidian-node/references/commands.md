# 工具命令

需要初始化、检查、交付或处理环境缺项时读取。命令适配 [主文档](../SKILL.md#适配版本) 中 CLI 0.3.0、Pack 0.13.0 和 Windows TestFlow 0.12.0；执行前核对已安装版本及帮助。

## 创建与交付

`cargo mires` 与 `cargo miresidian` 等价。除 new 从父目录创建外，其余操作通常在节点项目执行：

| 操作 | 命令 |
| --- | --- |
| 新目录 | `cargo miresidian new demo-node --name "演示节点" --miresidian-version 0.20.1` |
| 已有目录 | `cargo miresidian init --name "演示节点" --miresidian-version 0.20.1`（可指定已有目录） |
| 创建时指定框架 | new/init 使用 `--miresidian-version X.Y.Z`，等价别名 `--mires-version X.Y.Z` |
| 编译检查 | `cargo check --locked` |
| 结构/依赖/说明检查 | `cargo miresidian check` |
| 构建 x64 DLL | `cargo miresidian build --arch x64` |
| 构建并打包 x64 | `cargo miresidian pack --arch x64` |
| 生成烟测 BAT | `cargo miresidian testflow --os windows tools/smoke.testflow` |
| 静态检查流程 | `cargo miresidian-testflow-windows --check tools/smoke.testflow` |

- new 要求目标不存在、父目录存在；init 默认当前目录，目标已存在且模板文件/Cargo.lock 均不冲突，其他文件保留，模板子目录须为普通目录。冲突先检查已有内容，不覆盖。
- package 名取目录末段，`--name` 是独立显示名，用于 node 标注、说明文件和产物路径，须适合作为 Windows 文件名；省略时显示名也取目录名。
- 未指定框架时，创建查询最新未撤回正式版并精确锁定，生成真实锁文件；不代表编译或业务已完成，已有项目不自动升级。
- build/pack 默认所有发现节点和 all 架构，用 `--node "名称"`、`--arch x64/x86/all` 限定目标；均 release/locked，pack 已含构建，输出 `dist/<节点显示名>/`。manifest 反映本次构建，旧位数目录可能仍存在。
- check 不编译、不加载 DLL、不生成 dist；存在 tools/smoke.testflow 时一并静态检查，不能证明业务或文档正确。

## TestFlow

参数均为 JSON 字符串。以下为流程示意，须替换实际配置/操作；无业务配置用 Config()：

```text
GetFunctions()
GetInitConfig()
Config("连接地址", "1000")
InitDevice()
实际操作名("业务输入")
CloseDevice()
CloseAllDevices()
```

Delay("100") 单位毫秒，范围 0..2147483647；静态配置关联按原始字符串数组判断。涉及框架附加字段时按 GetInitConfig 组织参数。

生成不构建 DLL；Windows 用匹配架构的 DLL/PowerShell 执行 tools/smoke.bat，自动化设 `MIRESIDIAN_TESTFLOW_NO_PAUSE=1`，核对退出码与输出。BAT 按自身目录定位流程和 ../dist，保持相对结构。模板 Echo 不能替代真实业务烟测。

## Feature 检查

在依赖中固定交付能力，诊断可用：

```text
cargo tree --locked -e features -i miresidian
cargo check --locked --features miresidian/instances
```

只有节点定义并转发同名 feature 时才用 `--features instances`。CLI/Pack build、pack 没有 --features/--no-default-features，所需能力须在依赖或节点默认 features 中启用。

## 工具与环境缺项

仅缺工具或任务要求调整版本时安装，先确认目标版本已发布：`cargo install miresidian_cli --version 0.3.0 --locked`，再执行 `cargo miresidian install --full --pack-version 0.13.0 --testflow-version 0.12.0`。Cargo 管理用户级安装根目录，不建项目内工具目录；原生 cargo install 没有 --full。

查看版本用 `cargo miresidian list`；调整版本须在任务范围内，可用 `update --full` 更新后端或 `use pack 0.13.0` 切换指定版本。管理命令属于 cargo miresidian，CLI 本体由 cargo install 的 --version 指定。所选后端须满足 CLI 的最低要求；同一安装根替换当前版本，批量失败先核对部分结果。

Pack 目标为 x86_64-pc-windows-gnu / i686-pc-windows-gnu。复用现有 Rust、目标标准库和 MinGW 链接器，仅补本次目标缺项；rustup target add 不安装系统链接器，MSVC 环境不能替代 GNU 配置。Rust 版本满足框架及业务依赖 MSRV，模板 MSRV 为 1.89；环境受阻时说明具体阶段与缺项。
