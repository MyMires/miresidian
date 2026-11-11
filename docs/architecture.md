# 架构总览

本检出 0.30.1 的运行时适配接口为 `Node`，参数编解码和值模型使用 `NodeInput` / `NodeOutput` 等通用节点名称。源码接口迁移见[0.29.0](releases/0.29.0.md)。

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
    runtime --> instances["miresidian_instances：实例模型、容器与可选多通道调度"]
    instances --> codec
    runtime --> lease["miresidian_lease：通用租约模型"]
    lease --> codec
    runtime -->|format feature| format["miresidian_format：显式格式转换"]
    format --> codec
    runtime -->|batch feature| batch["miresidian_batch：记录解析与聚合"]
    batch --> codec
    macros -.-> generated["编译期生成：元数据、Node 实现与 DLL 导出"]
    generated -.-> node
    pack["miresidian_pack：开发工具"] -.-> dll["节点 DLL 与发布目录"]
    node -.-> dll
    testflow["miresidian_testflow_windows：开发工具"] -.-> bat["BAT / PowerShell 执行器"]
    cli["miresidian_cli：统一命令入口"] -.->|启动后端进程| pack
    cli -.->|启动后端进程| testflow
    bat -.-> dll
    host["C / C# / Python / LabVIEW 宿主"] -.-> dll
```

框架工作区包含八个库 crate（含根 crate、宏、ABI、codec、format、batch、instances、lease）和三个 Cargo 开发工具（CLI、Pack、TestFlow）；节点与烟测夹具是独立 Cargo 项目。`miresidian_macro` 在编译期运行，生成代码调用 `miresidian` 的公开 API，宏实现本身不进入 DLL 的运行时。CLI 启动独立后端进程，开发工具不进入节点 DLL 的运行时依赖图。

`instances` feature 与同名宏配置共同控制实例策略和多通道执行；未标注节点在任意 feature 组合下仍是单实例。根 crate 始终依赖独立 miresidian_instances 的通用模型及单实例容器，instances feature 开启其 runtime 后端；多实例容器和通用调度留在该 crate，根框架只适配节点生命周期。详见[实例管理](instances.md)。

根 crate 始终依赖租约 crate 的通用模型；`lease` feature 控制的是 `miresidian_lease/local-ipc` 与运行时的租约接入。只有启用 feature 且声明租约的节点才走租约路径。平台适配器和 ABI 支撑位于根 crate，具体通信依赖保留在独立节点项目。

依赖定义见[根 Cargo.toml](../Cargo.toml)与[租约 Cargo.toml](../features/lease/Cargo.toml)；接入步骤见[宏驱动开发](宏驱动开发.md)和[租约说明](../features/lease/README.md)。

## 核心类型关系（UML）

图中 `NodeRuntime`、`Instance`、`InstanceState` 省略泛型参数，它们实际承载同一节点类型 `N: Node`。`ExampleNode` 表示任意由宏适配的业务节点，并非框架内置类型。

```mermaid
classDiagram
    class Node["节点接口（Node）"] {
        <<trait>>
        +descriptor()
        +platform_registry()
        +validate_init(config, input)
        +init_device(config, input)
        +validate_definition()
        +invoke(operation, config, input)
        +close_device(config, input)
    }
    class ExampleNode["业务节点示例（ExampleNode）"] {
        业务状态与通信对象
    }
    class NodeRuntime["节点运行时（NodeRuntime）"] {
        -core
        -platforms
        +invoke(invocation)
        +invoke_with(adapter, request)
        +invoke_with_platform(platform, request)
        +shutdown()
    }
    class RuntimeState["共享运行时状态（RuntimeState）"] {
        management
        lifecycle
        stopping
        pending
        running
    }
    class Instance["节点实例（Instance）"] {
        <<内部类型>>
        key
        config
        execution
        state
        request
        pending
        closing
    }
    class InstanceState["实例状态（InstanceState）"] {
        <<内部类型>>
        node
        fault
    }
    class NodeAccess["节点访问分支（NodeAccess）"] {
        <<enum>>
        Direct(N)
        Shared(SharedNode)
    }
    class SharedNode["节点代理（SharedNode）"] {
        resource
    }
    class SharedResource["共享资源引用（SharedResource）"] {
        connection
        lease
    }
    class Owner["唯一节点所有者（Owner）"] {
        node
        fault
        output
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
        +format_parameter(parameter)
        +normalize_parameter(parameter, value)
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
        enum_options
    }
    note for Node "节点与运行时之间的接口，由属性宏生成实现。"
    note for NodeRuntime "不同实例并行；同实例请求和整个多通道批次互斥。"
    note for InstanceState "异步互斥锁保护可选对象状态；关闭时取走对象并完成清理。"
    note for NodeAccess "Direct 与 Shared 互斥；后者只持代理，不另构造本地 N。"
    note for Owner "所有者内构造一次 N；完整业务调用互斥，最后引用才关闭和析构。"
    note for PlatformAdapter "decode 解析平台请求；encode 编码平台输出。"
    Node <|.. ExampleNode : 宏生成实现
    NodeRuntime "1..*" o-- "1" RuntimeState : Clone 共享运行时身份
    RuntimeState "1" *-- "0..*" Instance : 默认最多一个；显式策略可多个
    Instance "1" *-- "0..1" InstanceState : Mutex 保护；关闭后取空
    InstanceState "1" *-- "1" NodeAccess : 访问分支
    NodeAccess "1" *-- "0..1" ExampleNode : Direct 持有 N
    NodeAccess "1" *-- "0..1" SharedNode : Shared 持有代理
    SharedNode "1" *-- "1" SharedResource : 资源会话引用
    SharedResource "1" *-- "1" LocalLease : 引用侧配置锁与会话
    SharedResource ..> Owner : IPC 转发完整业务
    Owner "1" *-- "0..1" ExampleNode : 初始化后持有唯一 N
    NodeRuntime "1" *-- "1" PlatformRegistry : 持有已注册平台
    PlatformRegistry "1" o-- "1..*" PlatformAdapter : Arc 共享
    PlatformAdapter <|.. TbLvTestPlatformAdapter : 实现接口
    Node ..> NodeDescriptor : 返回静态元数据
    NodeDescriptor "1" o-- "0..*" OperationDescriptor : 静态切片
    NodeDescriptor "1" o-- "0..*" ParameterDescriptor : 初始化与探测参数
    OperationDescriptor "1" o-- "0..*" ParameterDescriptor : 输入与输出
```

`+` 表示公开成员，`-` 表示私有成员；`trait` 表示 Rust 接口。关系中的 `1` 表示一个，`0..1` 表示可选，`0..*` 表示零到多个，`1..*` 表示至少一个。

`<|..` 表示 trait 实现，`*--` 表示对象持有，`o--` 表示共享引用或静态描述关联，`..>` 表示使用关系。`Instance` 与 `InstanceState` 是内部实现类型，不属于公开 Rust API。

### 图中成员的中文含义

方法和字段保留实际源码名称，下表补充中文对照；它们是职责摘要，完整参数与返回类型以源码为准。

| 所属类型 | 成员名称与含义 |
| --- | --- |
| 节点接口 `Node` | `descriptor` 查询节点描述；`platform_registry` 获取平台注册表；`init_device` 构造已初始化节点并返回业务输出；`validate_definition` 校验注册契约；`invoke` 执行业务操作；`close_device` 关闭节点实例 |
| 运行时 `NodeRuntime` | `core` 共享运行时状态；`platforms` 平台注册表；管理状态保存 objects 与 active_platform |
| 运行时调用方法 | `invoke` 按配置选择平台并调用；`invoke_with` 使用指定适配器调用；`invoke_with_platform` 按平台编号调用；`shutdown` 关闭全部通道 |
| 对象 `Instance` / `InstanceState` | `key` 关联键；`config` 已保存业务配置；`execution` 对象执行方式；`state` 加锁状态；`node` 为 Direct/Shared 分支；`fault` 为本地故障原因 |
| `NodeAccess` / `SharedNode` / `Owner` | Direct 持有本地 N；SharedNode 只持有资源会话引用；Owner 持有共享的 N、故障和初始化输出 |
| 租约 `LocalLease` | `validate` 校验连接及服务代次；`release` 释放租约与配置锁 |
| 平台注册与适配 | `register` 注册适配器；`adapter` 按平台查找适配器；`platform` 平台编号；`identifier` 文本标识；`decode` 解析请求；`encode` 编码响应 |
| 节点与操作描述 | `name` 名称；`init_config` 初始化配置；`operations` 操作清单；`inputs` 输入参数；`outputs` 输出参数；`channel_inputs` 共享调用使用的输入参数 |
| 参数描述 | `definition` 原样发布的描述文本；业务参数 `default_value` 为 None，格式选择器等框架项可有显式默认值 |

运行时默认只保留一个对象；instances 策略支持按业务 Config 组合键或 Handle 定位多个对象。默认全部业务 Config 组成实例键，可用 instance(key/ignore) 调整；任一选中原始键为空则跳过初始化。框架字段不进入业务配置。每个通道将节点状态与故障标记放在同一把锁中；业务 panic 使该通道进入仅可清理的状态。清理先关闭并析构节点，再释放租约。多通道调用同样以唯一一份 Config 定位一个对象，平台各通道输入均使用该对象；执行方式保存在各自对象上。`ExecutionMode` 只决定这些输入的调度方式，同一对象始终互斥执行；输入位置不关联对象创建顺序。具体输入与错误规则见[平台调用契约](c-abi.md#平台调用契约)。

Direct 清理本地 N；Shared 只释放当前代理，最后引用才清理唯一 N，随后释放所有者配置租约。实例状态不再直接保存 LocalLease；引用侧租约由 SharedResource 持有，所有者后端另持保护物理资源的租约。共享节点的 panic/故障保存在所有者中，不能通过另一引用绕过。

类型定义：[Node](../src/runtime/node.rs)、[NodeRuntime](../src/runtime/lifecycle/service.rs)、[Instance / NodeAccess / InstanceState](../src/runtime/lifecycle/instance.rs)、[SharedNode / Owner](../src/runtime/lifecycle/shared.rs)、[SharedResource](../features/lease/src/shared/client.rs)、[所有者后端](../features/lease/src/shared/server.rs)、[描述信息](../src/runtime/descriptor.rs)、[平台注册表与适配器](../src/platform.rs)、[LocalLease](../features/lease/src/local_ipc/client.rs)。

## 节点创建与方法汇总

0.28.0 / macro 0.24.0 在编译期将新旧 feature 声明归一为同一策略：裸 instances 为 Single/Config/Fixed(Sequential)，不声明时仍为 Single/Config/Disabled；裸 lease 为 Required，不声明仍为 Disabled。format 位置项和旧 formats 数组归一到相同有序列表及 selector。租约声明解析独立位于 macro/src/node/lease.rs，节点入口处理裸标记与分组，instances/format 各自检查默认、重复和混用。桥接生成、对象容器、资源代理和 ABI 均复用原实现；详见[宏简写迁移](releases/0.28.0.md)。

`node` 保留在结构体上，生成节点身份、同模块方法的类型绑定、Node 实现和唯一 C ABI 入口；`operation` 独立生成方法描述与包装，再提交注册项。每个节点模块只有一个具体节点，可以有多个固有 impl；未标注的方法不会进入平台契约。方法按源码位置稳定排序，首次请求校验重复和保留名称。

初始化不再依赖 Default：Direct 解析配置后本地构造；Lease 取得共享代理，由唯一所有者校验接口及完整初始化参数，首次请求才构造 N，后续请求复用初始化输出。取得节点或共享引用及输出后才登记通道。构造失败由构造函数回滚；返回 N 后的输出编码错误由框架关闭并析构该 N，共享所有者失败路径在清理后释放租约。端口探测是独立关联函数，不创建临时节点。

## 一次业务调用的时序

下图展示 ABI v10 的单 JSON 调用：宿主先核对版本，platform 在本次调用时指定，0/省略使用 DLL 固定默认平台，不从 Config 选型。唯一入口完成请求校验后，使用同一个实际适配器 decode、规范化枚举输入并渲染元数据，随后进入统一调度链。

```mermaid
sequenceDiagram
    autonumber
    participant Host as 测试平台（宿主）
    participant FFI as 跨语言入口<br/>ffi_runtime
    participant Runtime as 节点运行时<br/>NodeRuntime
    participant Adapter as 平台适配器<br/>PlatformAdapter
    participant Instance as 节点通道<br/>Instance
    participant Proxy as 可选共享代理<br/>SharedNode / SharedResource
    participant Node as 宏生成的节点接口<br/>Node 实现
    participant Codec as 数据编解码<br/>miresidian_codec
    participant Target as 节点业务逻辑与目标资源
    Host->>FFI: miresidian_invoke 提交单 JSON 请求
    Note over Host,FFI: 请求含 function、config/input 字符串数组及 platform；先核对 ABI v10
    FFI->>FFI: 校验文本与 JSON，构造请求（Invocation）
    FFI->>Runtime: 取得共享运行时并发起调用（按请求平台调用 invoke_with）
    Runtime->>Adapter: 解析平台请求（decode）
    Adapter-->>Runtime: 返回结构化命令（RuntimeInvocation）
    Runtime->>Adapter: 以类型候选规范化枚举输入（normalize_parameter）
    Runtime->>Instance: 短管理锁查找对象，释放后取得实例请求执行权
    Note over Runtime,Instance: 这里使用已初始化通道；框架保留配置不参与通道键
    opt Shared 分支
        Instance->>Proxy: 校验代理与配置租约（validate）
        Proxy-->>Instance: 确认会话仍有效
    end
    alt Direct 分支
        Instance->>Node: 本地按操作名分发（invoke）
    else Shared 分支
        Instance->>Proxy: 转发完整操作及上下文
        Proxy->>Node: 所有者中互斥分发（invoke）
    end
    Note over Node,Codec: 宏生成包装负责类型转换，业务方法接收 Rust 参数
    Node->>Codec: 将平台文本解析为强类型参数
    Codec-->>Node: 返回 Rust 参数值
    Node->>Target: 执行业务方法
    Target-->>Node: 返回业务结果
    Node->>Codec: 将业务结果编码为平台文本
    Codec-->>Node: 返回文本输出
    alt Direct 分支
        Node-->>Instance: 返回业务输出（InvocationOutput）
    else Shared 分支
        Node-->>Proxy: 所有者返回业务结果或错误
        Proxy-->>Instance: 恢复业务输出（InvocationOutput）
    end
    Instance-->>Runtime: 交回通道业务输出
    Runtime->>Runtime: 封装运行时响应（RuntimeResponse）
    Runtime->>Adapter: 编码平台响应（encode）
    Adapter-->>Runtime: 返回适配后的文本输出
    Runtime-->>FFI: 返回结果或统一错误
    FFI->>FFI: 组装状态、稳定代码、消息与业务输出
    Note over FFI: 对应字段依次为 status、code、message、output
    FFI-->>Host: 返回 DLL 当前线程持有的 JSON 指针
    Note over Host,FFI: 宿主立即复制文本；同一线程下一次调用前有效；不得释放
```

图中将通道查找与调度合并为一步；版本和元数据查询由运行时直接处理，不创建或调用节点实例。共享调用在适配器中保留唯一一份 Config 并解析各平台通道输入，运行时先按节点策略从 Config 组合键或 Handle 定位一个对象，再将所有有效输入分派给该对象。错误和 panic 路径的状态、稳定代码与恢复规则见 [C ABI](c-abi.md)，租约失效处理见[租约说明](../features/lease/README.md)。

Shared 分支的 Node 位于唯一资源所有者中；图中省略 IPC 编解码及所有者内部异步桥，不能把代理看作本地节点副本。这里的多通道“共享调用”是平台输入布局，与跨 DLL 的资源共享分别判断。

## 职责与源码位置

| 层次或目录 | 职责 | 主要入口 |
| --- | --- | --- |
| 根 `src/` | 公开入口与重导出 | [lib.rs](../src/lib.rs) |
| `src/runtime/` | 调用与描述模型、通道生命周期 | [contract.rs](../src/runtime/contract.rs)、[descriptor.rs](../src/runtime/descriptor.rs)、[service.rs](../src/runtime/lifecycle/service.rs) |
| `src/runtime/lifecycle/` | 单通道隔离与清理、平台选择和配置解析 | [instance.rs](../src/runtime/lifecycle/instance.rs)、[routing.rs](../src/runtime/lifecycle/routing.rs) |
| `features/instances/` | 通用实例策略、单/多实例容器和通用多通道调度 | [lib.rs](../features/instances/src/lib.rs) |
| `src/runtime/multi_channel.rs` | 同一对象的平台多通道输入校验与顺序/并行/广播调度 | [multi_channel.rs](../src/runtime/multi_channel.rs) |
| `src/platform/` | 平台专有输入输出映射 | [platform.rs](../src/platform.rs)、[TB 兼容适配器](../src/platform/tb_lv_test_platform.rs) |
| `src/interop/` | C 字符串与 JSON 边界、异常兜底、响应内存 | [ffi_runtime.rs](../src/interop/ffi_runtime.rs) |
| `abi/` | C ABI 常量、类型与唯一公共头文件 | [lib.rs](../abi/src/lib.rs)、[miresidian.h](../abi/api/miresidian.h) |
| `codec/` | 值模型、统一错误、参数与编解码 | [lib.rs](../codec/src/lib.rs) |
| `macro/` | 属性宏入口、节点声明与逐方法生成 | [lib.rs](../macro/src/lib.rs)、[node.rs](../macro/src/node.rs)、[operation.rs](../macro/src/operation.rs) |
| `macro/src/operation/` 与 `macro/src/node/` | 方法标注校验、描述及调用包装；节点注册绑定与 FFI | [definition.rs](../macro/src/operation/definition.rs)、[dispatch.rs](../macro/src/operation/dispatch.rs)、[generate.rs](../macro/src/node/generate.rs)、[ffi.rs](../macro/src/node/ffi.rs) |
| `features/format/` | 可选显式格式转换，宏生成选择输入及转换包装 | [说明](../features/format/README.md) |
| `features/batch/` | 按原类型解析批量记录、广播对齐及逐槽聚合，独立于 format | [说明](../features/batch/README.md) |
| `features/lease/` | 资源无关租约模型与可选本地 IPC | [lib.rs](../features/lease/src/lib.rs)、[local_ipc.rs](../features/lease/src/local_ipc.rs) |
| `tools/` | 统一命令路由、发布打包、Windows 流程执行及版本检查 | [CLI](../tools/miresidian-cli/README.md)、[打包说明](../tools/miresidian-pack/README.md)、[TestFlow 说明](../tools/miresidian-testflow-windows/README.md)、[check_versions.py](../tools/check_versions.py) |
| `tools/fixtures/smoke-node/` | 独立的工具链集成夹具 | [节点源码](../tools/fixtures/smoke-node/src/lib.rs) |
| `tests/` 与各 crate 的测试 | 跨 crate 契约、宏编译诊断与功能回归 | [根集成测试目录](../tests)、[宏诊断测试](../tests/macro_compile_fail.rs)、[编解码测试](../codec/tests) |
| 独立节点项目 | 资源状态、私有通信依赖、业务操作、节点测试与交付说明 | [宏驱动开发](宏驱动开发.md) |

源码使用新式 Rust 模块结构，不使用 `mod.rs`。入口文件负责声明与导出；同一概念具有多个协作职责时采用 `概念.rs + 概念/`，完整规则见[编码规范](coding-standards.md)。

修改时先按上表定位归属：节点标注与生成改 `macro/`，通用编解码改 `codec/`，平台协议映射改适配器，生命周期与共享调度改运行时，资源专有协议留在独立节点。涉及依赖、核心类型或调用链变化时同步更新本页。

## 并行与异步执行

0.15.0 的 `NodeRuntime` 使用共享内部状态；Clone 共享同一个实例登记表、平台绑定和并发限额。
宏生成的唯一 JSON DLL 调用入口使用一个 OnceLock 运行时，不再以全局 Mutex 包围完整调用。
管理锁只保护登记和查找；生命周期事务单独协调；实例请求门禁保护整个请求；状态锁保护业务对象与故障状态。

| 范围 | 执行规则 |
| --- | --- |
| 不同实例 | 可并行业务执行，受运行时并发预算限制 |
| 相同实例 | 请求互斥，多通道批次不可被其他请求插入 |
| 静态查询 | 不等待业务或生命周期事务 |
| 同步业务 | Tokio 阻塞池执行，最多 32 个阻塞线程 |
| 原生 async 业务 | 直接轮询 Future，挂起不占阻塞线程 |
| 初始化、关闭、探测 | 独立生命周期执行器，最多 4 个阻塞线程；当前同一运行时生命周期事务串行 |

默认 `ExecutionLimits` 为 running=32、pending=1024、pending_per_instance=64、batch=256。
pending 包含已开始和等待的请求，满容量直接返回错误；实例等待执行权时不占 running 预算。
实例请求门禁和全局运行预算使用 FIFO 异步信号量/互斥锁；生命周期和业务池分离，避免关闭等待业务时耗尽业务工作线程。
Parallel 多通道输入使用有界批次内的 Future 调度；每项仍通过状态锁，且全部已开始项结束后才返回错误。
独立 instances crate 的通用同步 Parallel 分发最多 32 个工作线程。

`invoke` 等同步 API 阻塞等待最终结果；`invoke_async`、`invoke_platform_async`、`invoke_command_async` 和 `invoke_batch_async` 供异步调用方等待。
宿主原生异步线程应使用 async API。同步 API 在多线程 Tokio 环境通过 blocking bridge 兼容，在单线程异步执行器中返回能力错误。
调用方丢弃等待 Future，不会中断已接收的框架请求；框架内部任务继续持有执行权直至安全结束。

`TaskService` 是有容量上限的任务所有者，提供 submit、submit_batch、poll、wait、cancel、release 和 shutdown / shutdown_async。
提交创建单调不复用的任务 ID；完成结果保留到显式释放，同样占容量；释放运行中任务会失败。
查询与等待可跨线程进行。等待超时仅返回当前快照，不取消任务。快照含 queued/running/cancelling/succeeded/failed/cancelled/timed_out 状态、取消请求标志、错误和输出。

`CallContext` 的截止时间从提交开始，涵盖排队。等待实例或执行预算时取消/到期可直接退出，不执行业务。
业务已经开始后采用协作取消，框架不强杀线程、不丢弃业务 Future、不提前关闭资源。
方法应将剩余时间传给 I/O 或检查 context；忽略取消的方法仍可返回成功，快照会保留取消请求标志。
初始化完成后若已取消或过期，构造对象先清理再返回错误，不登记对象。

关闭保持主键占位，拒绝新业务，等待在途请求后关闭/析构对象，再释放租约。
全局关闭包含已进入构造的初始化；业务 panic 隔离到对应实例，框架请求异常触发运行时清理。
显式关闭任务服务后，使用 `shutdown_executor` 等待并回收执行器线程；有在途请求时拒绝执行器关闭。
DLL 宿主必须在最终卸载前停止并发入口调用并完成清理。租约服务在 Windows 固定承载模块的已有约定仍然适用。

执行器依赖选用 Tokio 1.53.3 LTS（MIT，MSRV 1.71，支持 Linux/Windows），锁定在 1.53 补丁系列；futures-util 用于安全组合与捕获 Future panic。
依据 [Tokio 支持策略](https://docs.rs/crate/tokio/1.53.3) 与 [RUSTSEC-2026-0386 修复范围](https://rustsec.org/advisories/RUSTSEC-2026-0386.html) 选择已修复版本；本项目 MSRV 保持 1.89。

并发测试采用开始/释放信号证明真实执行重叠，覆盖默认与显式平台 JSON 请求、共享批次、关闭、初始化与全局关闭竞争、容量限制、排队取消、截止时间、原生异步 panic 及跨线程结果回收。

## 后续演进计划

本节集中记录尚未实现的需求；当前已实现机制见前文。历史测量与维护复盘见[历史归档](history/maintenance.md)。

### 国际化发布信息

- 若未来建设文档站点，再根据浏览器语言自动选择中文或英文页面；Git 提交信息与 Git 托管平台页面不支持自动切换。

### 节点流程测试工具

- `miresidian_testflow_windows` 将 `.testflow` 生成自包含 Windows BAT；流程解析、DLL C ABI 调用、返回格式化和自动 CloseDevice 均由框架内嵌执行器负责。
- 当 `.testflow` 的跨平台语法、断言和报告格式稳定后，再抽取 `miresidian_testflow` 核心 crate；届时 Windows BAT 执行器只保留平台加载职责。
- 工具应通过 C ABI 的 `GetFunctions` 与 `GetInitConfig` 自适应节点，而不包含串口业务规则。
- 保留节点项目对流程文件、测试数据和硬件环境的所有权。

### 待办：构建环境自动配置（未实现）

- [ ] 检测宿主操作系统/CPU、Rust/Cargo 版本，以及所选 DLL 目标平台和位数。
- [ ] 检查并按宿主平台处理 Rust target、GNU 链接器和相关系统构建依赖。
- [ ] 明确区分环境检查、自动配置与需要用户处理的系统组件，提供可执行诊断。
- [ ] 为 Windows/Linux 宿主及 Windows x64/x86 目标补齐对应验证场景。

当前 install --full 仅安装全部 Rust 配套工具，不代表完整构建环境已配置。
此项只记录后续需求，不纳入本次工具链发布。

### 待办：TestFlow 多通道测试（未实现，2026-11-01）

- [ ] 为 `.testflow` 增加多通道测试的调用方式和语法，明确各通道输入、通道选择及输出对应关系，并提供流程示例。
- [ ] 对接节点声明的顺序、并行和广播执行策略，区分多通道调用与多个设备实例的测试。
- [ ] 配套更新静态检查、BAT 内嵌执行器及使用说明，验证相关通道行为。

此项仅记录后续需求，具体语法待设计，暂不实现。

### 待办：自定义结构体的平台契约自动生成（未实现，2026-11-03）

当前自定义 struct 可通过手动实现 NodeInput/NodeOutput 或 FormatInput/FormatOutput 接入；输入侧按一个槽位整体解析，尚不能根据结构体字段自动展开多个平台槽位。现有 trait 实现不等于具备字段级自动映射能力。

- [ ] 设计结构体字段标注及 Input/Config 槽位展开，使框架从多个平台参数自动构造业务结构体，业务方法直接接收适合业务语义的类型。
- [ ] 设计结构体返回值的字段级 Output 展开，明确字段顺序、名称、输出数量和元数据的一致性。
- [ ] 明确字段默认值、可选字段、枚举/嵌套类型、缺失或非法字段的错误，以及不支持结构的编译期诊断。
- [ ] 评估通过 derive/属性宏自动生成自定义类型转换实现，区分单槽位整体转换与多槽位字段映射，不隐式猜测序列化格式。
- [ ] 明确与 format 格式选择、batch 记录边界及现有手写 trait 的组合和兼容性；保留旧节点依赖及既有默认解析行为。
- [ ] 配套提供输入构造、输出展开、默认值/错误和兼容性回归，并同步对应版本的节点开发文档、Rustdoc、示例与迁移说明；skill 仅在开发流程或资料查询方式确有变化时另行修订。

此项仅记录后续需求，标注语法与支持范围待设计，暂不实现。


## 关键契约与直接验收场景

框架行为以明确的执行次数、并发关系和输出位置验收，不能用实现内部一致或测试通过替代业务语义确认。

| 当前契约 | 直接验收证据 |
| --- | --- |
| 不同实例可同时执行业务；C 入口仍同步返回 | concurrent_execution：默认与显式平台 JSON 请求的执行开始信号必须重叠；同步返回不等于全 DLL 串行 |
| 相同 Handle 指向同一对象，不同 Handle 可指向独立对象 | concurrent_handles：相同业务配置的两个 Handle 同时进入业务 |
| 同实例整个多通道批次排斥其他请求 | concurrent_execution：批次业务轨迹之后才出现普通请求 |
| Broadcast 的源是通道1，业务仅调用一次 | broadcast：输入 [10,20,空,30] 输出 [R,R,空,R]，随后查询实际业务计数为1 |
| 空通道1不切换源；源失败不改用其他通道 | broadcast：后续有有效输入也必须保持业务0次或源失败1次 |
| 原生 async 等待时不占同步阻塞线程 | concurrent_execution：配置足够预算后，超过阻塞池线程数的 async 等待任务全部开始 |
| 取消是协作请求，不提前释放正在执行的实例 | concurrent_execution：排队任务不执行；运行中任务安全结束后才完成清理 |
| 执行器模块引用在线程退出后可释放 | module_lifetime 原生宿主：释放宿主引用、后台完成、关停、卸载和重新加载 |

广播的完整规范与迁移以[实例规范](instances.md#broadcast-的执行源与输出规则0160)为唯一详细定义；其他页面只引用或摘要该契约。


## 通信资源所有者

`lease::shared::SharedResource` 位于租约 crate 的可选本地 IPC 层，通信实现向其提供资源键、规范化连接配置、事务线协议和物理资源工厂。资源所有者持有设备连接并串行执行字节事务；各 DLL 的节点对象持有独立代理。框架运行时不接收串口等具体通信依赖，不跨 DLL 传 Rust 对象或指针。

原有实例容器仍按 Config 组合键/Handle 管理对象复用，原 LocalLease 仍校验配置与会话；资源代理额外管理唯一连接及代理引用。完整机制见[共享资源代理](shared-resources.md)。

## 0.25.0 Lease 节点所有权

InstanceState 通过 NodeAccess 区分 Direct(N) 与 Shared(SharedNode)。宏生成 LeaseRequest，运行时以 SharedNode 连接唯一所有者；所有者保存 N、初始化输出及接口信息，代理只保存 IPC 会话，不跨 DLL 传 Rust 对象或指针。共享条件、事务互斥与最后引用关闭由[共享资源专题](shared-resources.md)定义。
