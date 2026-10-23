# Changelog

本文记录 Miresidian 框架的已发布修订。格式遵循 Keep a Changelog，版本号遵循语义化版本。

## [Unreleased]

尚未发布的变更必须先记录在本节，发布时移动到对应版本与日期下。
## [0.4.1] - 2026-10-24

### Changed

- 将租约声明收敛为 `#[node("名称", lease(...))]` 子配置；拒绝旧的独立 `#[miresidian::lease]` 结构体属性，避免属性顺序和所有权语义分离。

## [0.4.0] - 2026-10-24

### Added

- 租约声明新增 `activation = required | disabled | selectable`；`selectable` 节点向平台发布资源访问方式，并在通道完整生命周期中保持所选 Lease 或 Direct 路径。

### Changed

- 框架及其公开子 crate 升级至 `0.4.0`；版本与修订记录必须随同一次发布变更维护。


## [0.3.1] - 2026-10-24

### Changed

- 移除节点重复声明的框架与 ABI 版本；`miresidian_pack` 改从 Cargo 实际解析的依赖图自动写入发布清单。


## [0.3.0] - 2026-10-24

### Added

- 拆分并公开 `miresidian_abi`、`miresidian_codec` 与 `miresidian_macro`；根 crate 保持统一入口，宏为必选能力。
- 新增 `miresidian_testflow_windows` 开发工具，用于从 `.testflow` 生成 Windows BAT 测试入口。
- 新增无硬件 Windows 冒烟夹具与 CI：安装正式 Cargo 子命令后，自动完成 x64 打包、BAT 生成及 DLL 调用。
- 为三个公开子 crate 补充包级 README，并明确 crates.io 首发顺序。

- `NodeRuntime` 按完整 Config 创建、复用、定位和关闭多个通道对象。
- 支持以 `Input[0] = "共享"` 对全部已创建通道执行自定义操作；空白或参数不符合的通道会跳过。
- 共享执行方式作为框架自动追加的初始化配置，支持 `Sequential-顺序`、`Parallel-并行` 与 `Broadcast-广播`；缺失、空白或无效值回退顺序执行。
- `Broadcast` 复用并校验一致的通道参数计划，仍为每个目标通道保留独立 Output 槽位。
- 参数描述中的 `实际值-显示值` 同时接受实际值与显示值；框架在 `DeviceInput` 前将显示值规范化为实际值。
- 版本标识统一为 `miresidian_version`、`miresidian_abi_version` 与 `miresidian` 发布清单键。

### Changed

- TestFlow Windows BAT 改为内嵌框架执行器，节点项目不再保存 PowerShell 调用器。
- TestFlow 执行器统一使用原生文件系统路径，兼容 UNC 路径下的 DLL 加载与流程读取；自动化可设置 `MIRESIDIAN_TESTFLOW_NO_PAUSE=1` 跳过暂停。

- 补充公开 crate、节点打包工具与 Windows TestFlow 工具的职责、输入输出及发布说明。

## [0.1.0] - 2026-10-21

### Added

- 建立框架版本、实例项目版本及 C ABI 版本的独立管理规则。
- 增加 `GetFrameworkVersion` 与 `GetProjectVersion` 常驻 ABI 查询。
- `cargo miresidian-pack` 发布包增加版本、Git 修订和 DLL SHA-256 清单。

### Compatibility

- C ABI: 5
