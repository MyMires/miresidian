# Changelog

本文记录 Miresidian 框架的已发布修订。格式遵循 Keep a Changelog，版本号遵循语义化版本。

## [Unreleased]

## [0.12.0] - 2026-10-29

### Changed

- `node` 保留在结构体上，移除 `node_impl`；`operation` 独立生成方法描述、参数转换及调用包装，支持同模块内多个固有 impl。初始化、关闭和可选端口探测同样标注 `operation`。
- 初始化改为无 self 的构造函数，返回 `MiresidianResult<Self>` 或 `MiresidianResult<(Self, T)>`；节点不再要求 Default，只有 T 进入平台输出。端口探测改为无需节点实例的关联函数。
- 保持 config、input、output 标注语法、参数来源、名称、默认值和编解码行为；保留初始化业务输出、通道复用、租约先取得后初始化，以及节点清理完成后才释放租约的顺序。
- 构造失败由构造过程回滚临时资源；已返回节点在初始化输出编码或租约校验失败时由框架关闭及析构。可展开 panic 继续在节点与 FFI 边界隔离。
- 新增 inventory 0.3.24（MIT OR Apache-2.0）作为内部方法注册后端；成熟库负责目标平台的链接保留与元数据注册，避免依赖源码扫描、宏展开顺序或编译进程全局状态。注册时不执行业务或设备 I/O，接口封装为非稳定宏支持项。
- 方法按源码位置稳定排序，运行时拒绝重复或保留的平台操作名称；条件编译的方法不会遗留注册项。当前每个节点模块只声明一个具体节点，节点状态与平台方法位于同一模块。
- 公开 Rust API 与生命周期契约按 pre-1.0 次版本升级，同步框架发布列车、独立烟测夹具、锁文件及示例。C ABI 保持 v7，租约协议保持 v2。

### Validation

- 默认/全特性工作区 Clippy 与全部测试、Rustdoc 示例、Release 构建、Rust 1.89 全目标全特性编译及生命周期/租约回归通过。
- Release+LTO Linux 动态库实际加载，验证两个 C 入口、版本、方法注册、中文回显及关闭；真实重命名依赖节点与串口节点契约测试通过。
- Windows x64/x86 全目标全特性交叉检查通过；CI 新增双位数 DLL 构建和实际注册分发烟测，当前 Linux 环境未执行 Windows DLL 原生加载。

## [0.11.2] - 2026-10-29

### Changed

- 架构图补充中英双语类型名称、中文职责注释与成员对照，时序图按中文步骤说明请求解析、通道调用、编解码和响应生命周期。
- 将 docs 从 11 个文件整合为 6 个：平台调用契约和适配器并入 C ABI，编解码并入节点开发，构建环境并入编码规范，演进计划并入架构总览；同步站内引用并记录旧文档对应章节。
- 文档交付调整按补丁版本发布，内部精确依赖、锁文件和烟测夹具版本同步；公开 Rust API、C ABI v7 与租约协议 v2 保持不变。

## [0.11.1] - 2026-10-29

### Changed

- 建立统一文档中心，按平台接入、节点开发和框架维护组织阅读路径与完整索引；精简项目首页，专题文档补充返回导航。
- 将项目结构说明集中到架构总览，增加 crate 依赖图、核心类型 UML 类图、调用时序图及源码职责索引；将租约接入示例归并到租约说明。
- 文档交付整理按补丁版本发布，同步内部精确依赖和锁文件；公开 Rust API、C ABI v7 与租约协议 v2 均无变更。

## [0.11.0] - 2026-10-29

### Added

- 补齐已约定的 `status、code、message、output` 响应契约：成功代码为 `OK`，错误由 `MiresidianError::category()` 和 `code()` 集中分类，两类 C ABI 入口及异常兜底统一构造响应。
- 增加端到端响应契约回归，覆盖字段顺序、版本与业务输出、平台错误、非法 UTF-8/JSON、共享上下文和 panic。
- 新增公开错误类别与类型化错误，按 pre-1.0 Rust API 规则将整个框架发布列车升级至 0.11.0；C ABI 保持 v7。
- 将版本管理提升为 P0 提交门禁，增加指导文件、版本检查脚本和提交/标签钩子。

### Changed

- 最低 Rust 版本修正为 1.89；CI 在 Linux/Windows 验证最低版本和 P1 异常恢复回归。
- 租约升级为 v2：增加服务代次、原连接验证和有界 IPC 请求；客户端配置文件锁在服务宿主退出后仍阻止不兼容配置。升级须先停止所有 v1 节点进程。
- 节点 panic 按通道隔离；故障对象仅允许清理，关闭后重新初始化。关闭、析构或租约释放失败时仍移除通道，全部清理路径先析构节点再释放租约。

- `#[miresidian::operation]` 仅接受可选的平台名称；共享调用策略统一由 `InitDevice` 的 `共享执行方式` 配置决定。
- 共享调用的输出槽位改为与请求输入槽位严格一一对齐；未提供输入的后续通道不再产生空输出槽位，并行模式只为有效输入创建任务。
- `PlatformRegistry` 替换默认平台的文本标识时，会同步更新默认平台数值，避免后续默认调用指向未注册适配器。
- C ABI 头文件收敛为 `miresidian_abi` crate 中的 `abi/api/miresidian.h` 单一来源；根 crate 的 C 头文件测试改为验证该文件，且移除重复的 ABI 版本宏定义。

### Fixed

- 租约崩溃回归使用独立服务会话，并在子进程中隔离环境，避免与并行验证或真实服务共用宿主；配置锁仍跨服务命名空间保护资源。
- 补上早前生命周期优化中遗漏的稳定 code 字段；共享错误增加通道上下文时保留原错误分类，Windows TestFlow 显示 code 并兼容旧 DLL 缺少该字段的响应。
- 修复业务 panic 污染全局运行时锁后，版本查询、其他通道和清理入口持续失败的问题；外部污染的锁先清理再恢复。
- 修复服务宿主退出后，存活租约的配置约束被新服务绕过的问题，并增加真实多进程回归。
- Windows 租约服务使用 `PIN | FROM_ADDRESS` 固定承载 DLL。

### Removed

- 删除未纳入根 crate 编译图的遗留 `foundation.rs` 和重复 C ABI 头文件副本。

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
