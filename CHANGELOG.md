# Changelog

本文记录 Miresidian 框架的已发布修订。格式遵循 Keep a Changelog，版本号遵循语义化版本。

## [Unreleased]

## [0.10.0] - 2026-10-26

### Added

- 新增可扩展 Rust `Platform` 枚举：内置 `TbLvTestPlatform` 与第三方 `Custom(u32)`；适配器与注册表以该类型定位平台。

### Changed

- C ABI 升级至 v7；`miresidian_invoke_with` 的 `platform` 从 UTF-8 文本标识改为 `uint32_t` 平台枚举值。

## [0.9.0] - 2026-10-26

### Added

- C ABI 新增 `miresidian_invoke_with(platform, function, config_json, input_json)`；由节点注册表解析平台 id，并走 `NodeRuntime::invoke_with` 的显式适配路径。

### Changed

- C ABI 升级至 v6；宏生成的每个节点 DLL 同时导出显式平台入口。

## [0.8.0] - 2026-10-26

### Added

- 新增 `NodeRuntime::shutdown()` 与固定操作 `CloseAllDevices`：逐一尽力关闭全部通道，释放完成后统一重置通道、平台适配器与共享执行状态。

### Fixed

- `CloseDevice` 在节点已成功关闭后即移除通道；即使后续租约释放失败，也不会留下不可用的活动通道。
- C ABI 现在将参数校验与运行时明确报告的未知操作映射为状态码 `1`，将未初始化、通道不存在与节点执行失败映射为状态码 `2`。

## [0.7.0] - 2026-10-26

### Added

- 新增平台适配抽象 `PlatformAdapter`、平台无关的结构化共享调用/响应模型，以及当前 `tb_lv_test_platform` 的兼容适配器。
- 新增 `PlatformRegistry`；普通 `invoke` 可由 `InitDevice` 的保留 Config 在已注册适配器中选择平台，`invoke_with(adapter, ...)` 则保持显式选型且不发布该 Config。
- `#[node(..., platforms(AdapterType, ...))]` 可将实现 `PlatformAdapter + Default` 的适配器编译进节点 DLL；C ABI 首次调用使用该注册表，外部可通过原有 `InitDevice` Config 实际选中平台。

### Changed

- 共享调度结果现在始终与已初始化通道数对齐；不足输入槽位补为跳过，超过通道数的槽位被明确拒绝。

## [0.6.0] - 2026-10-26

### Fixed

- 宏生成的 `InitDevice` 元数据现在将 `Config` 与 `Input` 参数分开：仅前者构成通道键，框架保留的共享执行方式可在混合参数初始化中正确解析；`GetFunctions` 仍发布初始化 Input 参数。
- 通道键现在按参数定义补齐默认值、移除 `名称:` 前缀并规范化选项显示值；等价业务 Config 可复用和定位同一通道。

## [0.5.3] - 2026-10-24

### Fixed

- 未启用 `lease` feature 时不再生成仅供租约生命周期使用的 `Channel::lease` 字段，默认构建保持无 unused/dead-code 告警。

## [0.5.2] - 2026-10-24

### Added

- `lease` feature 新增由本地 IPC 服务统一维护的跨 DLL 租约注册表：Windows 使用命名管道，Unix 使用本地 socket；兼容配置可共享，不兼容配置跨运行时一致拒绝，连接断开自动回收租约。

### Changed

- `NodeRuntime` 不再持有每个 DLL 私有的租约注册表，改由框架本地租约服务分配和释放令牌。

## [0.5.1] - 2026-10-24

### Fixed

- `miresidian_pack` 解析 `#[node]` 时保留节点名称并接受 `lease(...)` 等子配置，可重新打包启用租约的节点 DLL。
## [0.5.0] - 2026-10-24



### Changed

- 租约令牌与兼容性校验完全收敛至 `NodeRuntime`；删除业务节点的 `lease_acquire`、`lease_release` 和 `LeaseParticipant` 回调接口。
- `init_device` 与 `close_device` 恢复为纯通信生命周期方法；宏负责从其初始化参数生成租约请求，框架负责获取、失败回滚与释放。

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
