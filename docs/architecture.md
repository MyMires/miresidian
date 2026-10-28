# 架构总览

[文档中心](README.md) · [项目首页](../README.md)

本文从依赖、核心类型和调用过程三个角度描述当前实现。图表使用 Mermaid，支持 Mermaid 的 Markdown 阅读器可直接渲染；纯文本环境可结合下方职责表和源码链接阅读。图表是人工维护的设计视图，省略辅助类型和次要方法；精确字段、泛型约束和签名以链接的源码及 Rustdoc 为准。图中使用中文说明职责，括号内保留源码标识符，便于搜索定位。

## crate 与节点边界

实线箭头表示框架库的直接 Cargo 依赖；虚线表示编译期生成或构建、交付关系，不表示节点 DLL 的运行时依赖。

```mermaid
flowchart TB
    node["独立节点项目：状态、通信与业务方法"] --> runtime["miresidian：运行时与公开重导出"]
    node --> transport["节点私有通信依赖：串口 / TCP / USB / CAN"]
    runtime --> abi["miresidian_abi：C ABI 契约"]
    runtime --> codec["miresidian_codec：解析、编解码与值模型"]
    runtime --> macros["miresidian_macro：过程宏"]
    runtime --> registry["inventory：内部方法注册与链接保留"]
    runtime --> lease["miresidian_lease：通用租约模型"]
    lease --> codec
    macros -.-> generated["编译期生成：元数据、DeviceNode 实现与 DLL 导出"]
    generated -.-> node
    pack["miresidian_pack：开发工具"] -.-> dll["节点 DLL 与发布目录"]
    node -.-> dll
    testflow["miresidian_testflow_windows：开发工具"] -.-> bat["BAT / PowerShell 执行器"]
    bat -.-> dll
    host["C / C# / Python / LabVIEW 宿主"] -.-> dll
```

框架工作区包含五个库 crate 和两个 Cargo 开发工具；节点与烟测夹具是独立 Cargo 项目。`miresidian_macro` 在编译期运行，生成代码调用 `miresidian` 的公开 API，宏实现本身不进入 DLL 的运行时。开发工具通过构建产物或 C ABI 工作，也不进入节点 DLL 的运行时依赖图。

根 crate 始终依赖租约 crate 的通用模型；`lease` feature 控制的是 `miresidian_lease/local-ipc` 与运行时的租约接入。只有启用 feature 且声明租约的节点才走租约路径。平台适配器和 ABI 支撑位于根 crate，具体通信依赖保留在独立节点项目。

依赖定义见[根 Cargo.toml](../Cargo.toml)与[租约 Cargo.toml](../lease/Cargo.toml)；接入步骤见[宏驱动开发](宏驱动开发.md)和[租约说明](../lease/README.md)。

## 核心类型关系（UML）

图中 `NodeRuntime`、`Channel`、`ChannelState` 省略泛型参数，它们实际承载同一节点类型 `N: DeviceNode`。`ExampleNode` 表示任意由宏适配的业务节点，并非框架内置类型。

```mermaid
classDiagram
    class DeviceNode["节点接口（DeviceNode）"] {
        <<trait>>
        +descriptor()
        +platform_registry()
        +init_device(config, input)
        +validate_definition()
        +invoke(operation, config, input)
        +close_device(config, input)
    }
    class ExampleNode["业务节点示例（ExampleNode）"] {
        业务状态与通信对象
    }
    class NodeRuntime["节点运行时（NodeRuntime）"] {
        -channels
        -platforms
        -active_platform
        -shared_execution
        +invoke(invocation)
        +invoke_with(adapter, request)
        +invoke_with_platform(platform, request)
        +shutdown()
    }
    class Channel["设备通道（Channel）"] {
        <<内部类型>>
        config
        state
    }
    class ChannelState["通道状态（ChannelState）"] {
        <<内部类型>>
        node
        fault
        lease
    }
    class LocalLease["本地资源租约（LocalLease）"] {
        +validate()
        +release()
    }
    class PlatformRegistry["平台注册表（PlatformRegistry）"] {
        +register(adapter)
        +adapter(platform)
    }
    class PlatformAdapter["平台适配接口（PlatformAdapter）"] {
        <<trait>>
        +platform()
        +identifier()
        +decode(request)
        +encode(response)
    }
    class TbLvTestPlatformAdapter["TB 平台适配器（TbLvTestPlatformAdapter）"]
    class NodeDescriptor["节点描述（NodeDescriptor）"] {
        name
        init_config
        operations
    }
    class OperationDescriptor["操作描述（OperationDescriptor）"] {
        name
        inputs
        outputs
        channel_inputs
    }
    class ParameterDescriptor["参数描述（ParameterDescriptor）"] {
        name
        definition
        default_value
    }
    note for DeviceNode "节点与运行时之间的接口，由属性宏生成实现。"
    note for NodeRuntime "管理多通道并选择平台；invoke 发起调用，shutdown 关闭全部通道。"
    note for ChannelState "节点、故障原因和可选租约由同一把互斥锁保护。"
    note for PlatformAdapter "decode 解析平台请求；encode 编码平台输出。"
    DeviceNode <|.. ExampleNode : 宏生成实现
    NodeRuntime "1" *-- "0..*" Channel : 按 Config 管理
    Channel "1" *-- "1" ChannelState : Mutex 保护
    ChannelState "1" *-- "1" ExampleNode : 持有 N
    ChannelState "1" *-- "0..1" LocalLease : feature 与声明启用时
    NodeRuntime "1" *-- "1" PlatformRegistry : 持有已注册平台
    PlatformRegistry "1" o-- "1..*" PlatformAdapter : Arc 共享
    PlatformAdapter <|.. TbLvTestPlatformAdapter : 实现接口
    DeviceNode ..> NodeDescriptor : 返回静态元数据
    NodeDescriptor "1" o-- "0..*" OperationDescriptor : 静态切片
    NodeDescriptor "1" o-- "0..*" ParameterDescriptor : 初始化与探测参数
    OperationDescriptor "1" o-- "0..*" ParameterDescriptor : 输入与输出
```

`+` 表示公开成员，`-` 表示私有成员；`trait` 表示 Rust 接口。关系中的 `1` 表示一个，`0..1` 表示可选，`0..*` 表示零到多个，`1..*` 表示至少一个。

`<|..` 表示 trait 实现，`*--` 表示对象持有，`o--` 表示共享引用或静态描述关联，`..>` 表示使用关系。`Channel` 与 `ChannelState` 是内部实现类型，不属于公开 Rust API。

### 图中成员的中文含义

方法和字段保留实际源码名称，下表补充中文对照；它们是职责摘要，完整参数与返回类型以源码为准。

| 所属类型 | 成员名称与含义 |
| --- | --- |
| 节点接口 `DeviceNode` | `descriptor` 查询节点描述；`platform_registry` 获取平台注册表；`init_device` 构造已初始化节点并返回业务输出；`validate_definition` 校验注册契约；`invoke` 执行业务操作；`close_device` 关闭设备 |
| 运行时 `NodeRuntime` | `channels` 通道集合；`platforms` 平台注册表；`active_platform` 当前平台；`shared_execution` 共享执行方式 |
| 运行时调用方法 | `invoke` 按配置选择平台并调用；`invoke_with` 使用指定适配器调用；`invoke_with_platform` 按平台编号调用；`shutdown` 关闭全部通道 |
| 通道 `Channel` / `ChannelState` | `config` 规范化通道键；`state` 加锁状态；`node` 业务对象；`fault` 故障原因；`lease` 可选租约 |
| 租约 `LocalLease` | `validate` 校验连接及服务代次；`release` 释放租约与配置锁 |
| 平台注册与适配 | `register` 注册适配器；`adapter` 按平台查找适配器；`platform` 平台编号；`identifier` 文本标识；`decode` 解析请求；`encode` 编码响应 |
| 节点与操作描述 | `name` 名称；`init_config` 初始化配置；`operations` 操作清单；`inputs` 输入参数；`outputs` 输出参数；`channel_inputs` 共享调用使用的输入参数 |
| 参数描述 | `definition` 原样发布的描述文本；`default_value` 可选默认值 |

运行时以规范化后的业务 Config 定位通道，框架保留配置不进入通道键。每个通道将节点状态与故障标记放在同一把锁中；业务 panic 使该通道进入仅可清理的状态。清理先关闭并析构节点，再释放租约。共享调用的分发策略由 `ExecutionMode` 决定，具体槽位与错误规则见[平台调用契约](c-abi.md#平台调用契约)。

类型定义：[DeviceNode](../src/runtime/node.rs)、[NodeRuntime](../src/runtime/lifecycle.rs)、[Channel / ChannelState](../src/runtime/lifecycle/channel.rs)、[描述信息](../src/runtime/descriptor.rs)、[平台注册表与适配器](../src/platform.rs)、[LocalLease](../lease/src/local_ipc/client.rs)。

## 节点创建与方法汇总

`node` 保留在结构体上，生成节点身份、同模块方法的类型绑定、DeviceNode 实现和唯一 C ABI 入口；`operation` 独立生成方法描述与包装，再提交注册项。每个节点模块只有一个具体节点，可以有多个固有 impl；未标注的方法不会进入平台契约。方法按源码位置稳定排序，首次请求校验重复和保留名称。

初始化不再依赖 Default：解析配置并取得租约后，调用关联构造函数，成功得到节点与业务输出后才登记通道。构造失败由构造函数内部回滚；节点已返回后，输出编码或租约校验失败由框架关闭并析构，再释放租约。端口探测是独立关联函数，不创建临时节点。普通业务调用、共享策略与 C ABI 仍遵循原有契约。

## 一次业务调用的时序

下图按编号展示已初始化通道上的成功调用：先解析请求与选择通道，再执行业务，最后编码响应。`miresidian_invoke_with` 额外选择已注册的平台；默认入口通过运行时配置选择平台。两者在适配完成后进入同一调度链。

```mermaid
sequenceDiagram
    autonumber
    participant Host as 测试平台（宿主）
    participant FFI as 跨语言入口<br/>ffi_runtime
    participant Runtime as 节点运行时<br/>NodeRuntime
    participant Adapter as 平台适配器<br/>PlatformAdapter
    participant Channel as 设备通道<br/>Channel
    participant Lease as 可选资源租约<br/>LocalLease
    participant Node as 宏生成的节点接口<br/>DeviceNode 实现
    participant Codec as 数据编解码<br/>miresidian_codec
    participant Device as 节点业务方法与设备
    Host->>FFI: 提交函数名、配置数组与输入数组
    Note over Host,FFI: function 为函数名；config_json 和 input_json 为 JSON 字符串数组
    FFI->>FFI: 校验文本与 JSON，构造请求（Invocation）
    FFI->>Runtime: 加锁并发起调用（invoke / invoke_with）
    Runtime->>Adapter: 解析平台请求（decode）
    Adapter-->>Runtime: 返回结构化命令（RuntimeInvocation）
    Runtime->>Channel: 按规范化配置找到通道并执行
    Note over Runtime,Channel: 这里使用已初始化通道；框架保留配置不参与通道键
    opt 通道已获取资源租约
        Channel->>Lease: 校验租约（validate）
        Lease-->>Channel: 确认连接与服务代次仍有效
    end
    Channel->>Node: 按操作名分发业务（invoke）
    Note over Node,Codec: 宏生成包装负责类型转换，业务方法接收 Rust 参数
    Node->>Codec: 将平台文本解析为强类型参数
    Codec-->>Node: 返回 Rust 参数值
    Node->>Device: 执行设备业务方法
    Device-->>Node: 返回业务结果
    Node->>Codec: 将业务结果编码为平台文本
    Codec-->>Node: 返回文本输出
    Node-->>Channel: 返回业务输出（InvocationOutput）
    Channel-->>Runtime: 交回通道业务输出
    Runtime->>Runtime: 封装运行时响应（RuntimeResponse）
    Runtime->>Adapter: 编码平台响应（encode）
    Adapter-->>Runtime: 返回适配后的文本输出
    Runtime-->>FFI: 返回结果或统一错误
    FFI->>FFI: 组装状态、稳定代码、消息与业务输出
    Note over FFI: 对应字段依次为 status、code、message、output
    FFI-->>Host: 返回 DLL 当前线程持有的 JSON 指针
    Note over Host,FFI: 宿主立即复制文本；同一线程下一次调用前有效；不得释放
```

图中将通道查找与调度合并为一步；版本和元数据查询由运行时直接处理，不创建或调用设备节点。共享调用在适配器中转为结构化槽位，再由运行时分派到多个通道。错误和 panic 路径的状态、稳定代码与恢复规则见 [C ABI](c-abi.md)，租约失效处理见[租约说明](../lease/README.md)。

## 职责与源码位置

| 层次或目录 | 职责 | 主要入口 |
| --- | --- | --- |
| 根 `src/` | 公开入口与重导出 | [lib.rs](../src/lib.rs) |
| `src/runtime/` | 调用与描述模型、通道生命周期 | [contract.rs](../src/runtime/contract.rs)、[descriptor.rs](../src/runtime/descriptor.rs)、[lifecycle.rs](../src/runtime/lifecycle.rs) |
| `src/runtime/lifecycle/` | 单通道隔离与清理、平台选择和配置解析 | [channel.rs](../src/runtime/lifecycle/channel.rs)、[routing.rs](../src/runtime/lifecycle/routing.rs) |
| `src/runtime/shared.rs` | 多通道参数校验、顺序/并行/广播分发 | [shared.rs](../src/runtime/shared.rs) |
| `src/platform/` | 平台专有输入输出映射 | [platform.rs](../src/platform.rs)、[TB 兼容适配器](../src/platform/tb_lv_test_platform.rs) |
| `src/interop/` | C 字符串与 JSON 边界、异常兜底、响应内存 | [ffi_runtime.rs](../src/interop/ffi_runtime.rs) |
| `abi/` | C ABI 常量、类型与唯一公共头文件 | [lib.rs](../abi/src/lib.rs)、[miresidian.h](../abi/api/miresidian.h) |
| `codec/` | 值模型、统一错误、参数与编解码 | [lib.rs](../codec/src/lib.rs) |
| `macro/` | 属性宏入口、节点声明与逐方法生成 | [lib.rs](../macro/src/lib.rs)、[node.rs](../macro/src/node.rs)、[operation.rs](../macro/src/operation.rs) |
| `macro/src/operation/` 与 `macro/src/node/` | 方法标注校验、描述及调用包装；节点注册绑定与 FFI | [definition.rs](../macro/src/operation/definition.rs)、[dispatch.rs](../macro/src/operation/dispatch.rs)、[generate.rs](../macro/src/node/generate.rs)、[ffi.rs](../macro/src/node/ffi.rs) |
| `lease/` | 设备无关租约模型与可选本地 IPC | [lib.rs](../lease/src/lib.rs)、[local_ipc.rs](../lease/src/local_ipc.rs) |
| `tools/` | 发布打包、Windows 流程执行及版本检查 | [打包说明](../tools/miresidian-pack/README.md)、[TestFlow 说明](../tools/miresidian-testflow-windows/README.md)、[check_versions.py](../tools/check_versions.py) |
| `tools/fixtures/smoke-node/` | 独立的工具链集成夹具 | [节点源码](../tools/fixtures/smoke-node/src/lib.rs) |
| `tests/` 与各 crate 的测试 | 跨 crate 契约、宏编译诊断与功能回归 | [根集成测试目录](../tests)、[宏诊断测试](../tests/macro_compile_fail.rs)、[编解码测试](../codec/tests) |
| 独立节点项目 | 设备状态、私有通信依赖、业务操作、节点测试与交付说明 | [宏驱动开发](宏驱动开发.md) |

源码使用新式 Rust 模块结构，不使用 `mod.rs`。入口文件负责声明与导出；同一概念具有多个协作职责时采用 `概念.rs + 概念/`，完整规则见[编码规范](coding-standards.md)。

修改时先按上表定位归属：节点标注与生成改 `macro/`，通用编解码改 `codec/`，平台协议映射改适配器，生命周期与共享调度改运行时，设备协议留在独立节点。涉及依赖、核心类型或调用链变化时同步更新本页。

## 后续演进计划

本节记录已确认、但尚未进入当前发布版本的工程化事项。

### 国际化发布信息

- 提交说明优先使用中文。
- 公开发布前，为 README、CHANGELOG 和 Release Notes 提供中英文版本。
- 若未来建设文档站点，再根据浏览器语言自动选择中文或英文页面；Git 提交信息与 Git 托管平台页面不支持自动切换。

### 节点流程测试工具

- `miresidian_testflow_windows` 将 `.testflow` 生成自包含 Windows BAT；流程解析、DLL C ABI 调用、返回格式化和自动 CloseDevice 均由框架内嵌执行器负责。
- 当 `.testflow` 的跨平台语法、断言和报告格式稳定后，再抽取 `miresidian_testflow` 核心 crate；届时 Windows BAT 执行器只保留平台加载职责。
- 工具应通过 C ABI 的 `GetFunctions` 与 `GetInitConfig` 自适应节点，而不包含串口业务规则。
- 保留节点项目对流程文件、测试数据和硬件环境的所有权。
