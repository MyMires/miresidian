# 平台接入与 C ABI

[文档中心](README.md) · [架构总览](architecture.md)

本文集中说明节点 DLL 的跨语言入口、TB LV Test Platform 调用契约和平台适配扩展。宿主集成先读 C ABI，平台调用规则见第二章，新增适配器见第三章。

**章节导航：[C ABI](#c-abi) · [平台调用契约](#平台调用契约) · [平台适配器](#平台适配器)**

## C ABI

Miresidian 节点 DLL 导出默认入口 `miresidian_invoke` 与显式平台入口 `miresidian_invoke_with`。两个入口均使用 UTF-8 JSON 文本，
不要求调用方分配输出缓冲区，不传递 Rust 结构体。

ABI 版本 miresidian_abi_version() 当前返回 7。

### 入口声明

~~~c
uint32_t miresidian_abi_version(void);

const char *miresidian_invoke(
    const char *function,
    const char *config_json,
    const char *input_json);

const char *miresidian_invoke_with(
    uint32_t platform,
    const char *function,
    const char *config_json,
    const char *input_json);
~~~

- platform：仅 `miresidian_invoke_with` 使用；`uint32_t` 平台枚举值。`1` 为 `tb_lv_test_platform`，第三方平台使用不小于 `65536` 的已注册值。
- function：UTF-8 C 字符串，例如 GetFunctions、GetFrameworkVersion、GetProjectVersion、InitDevice、CloseAllDevices 或节点操作名。
- config_json、input_json：JSON 字符串数组，例如 ["COM3","9600"]；传 NULL 等同于 []。数组元素不能是数值、布尔值或对象。
- 返回值 `result_json`：DLL 内部保存的 UTF-8 JSON C 字符串指针。

DLL 在当前线程中持有返回文本。调用方无需分配输出缓冲区、无需传容量、也不得释放返回指针；
应在同一线程下一次调用前将文本复制到自己的内存。

默认入口按 `InitDevice` 的保留 `平台适配` Config 选择平台。显式平台入口对应 Rust 的 `NodeRuntime::invoke_with`：平台枚举数值单独传入，`InitDevice` Config 不发布也不接受 `平台适配`。

例如，调用 GetFunctions：

~~~c
const char *result_json = miresidian_invoke(
    "GetFunctions", "[]", "[]");
~~~

### 响应体

返回文本始终是同一种 JSON 对象，字段顺序固定为 status、code、message、output。

成功时：

~~~json
{
  "status": 0,
  "code": "OK",
  "message": "",
  "output": ["业务输出1", "业务输出2"]
}
~~~

失败时：

~~~json
{
  "status": 2,
  "code": "NOT_INITIALIZED",
  "message": "设备尚未初始化",
  "output": []
}
~~~

status 为数值型状态分类；code 为稳定机器代码，成功时为 `OK`；message 为成功时的空字符串或失败诊断；output 始终为业务字符串数组。调用方应以 status 和 code 判断结果，不应解析 message 文本。
JSON 转义、中文与换行由标准 JSON 编解码器处理，调用方不得自行按逗号拆分文本。

GetFunctions、GetFrameworkVersion、GetProjectVersion、GetInitConfig 和 GetPortNO 是常驻查询；它们不要求先调用 InitDevice。`CloseAllDevices` 也不要求先初始化：它会幂等地清理当前运行时持有的全部通道。
GetFrameworkVersion 的 output 为当前 DLL 所用框架的语义化版本；GetProjectVersion 的 output 为当前 DLL 项目的语义化版本。
这两个值来自 DLL 的编译期元数据，可用于与发布包 `manifest.json` 交叉核对。
只有节点定义的设备操作才会在未初始化时返回“设备尚未初始化”。

### 状态码

| 值 | 含义 |
| --- | --- |
| 0 | 成功 |
| 1 | 参数无效：ABI 文本/JSON 不合法，或运行时拒绝参数、运行时明确报告的未知操作。 |
| 2 | 操作失败：未初始化、通道不存在、节点关闭或业务执行失败。 |
| 3 | 内部错误 |

### 异常隔离与恢复

节点的可展开 panic 返回 `status = 3`。业务操作 panic 后对应通道被标记为故障，后续操作和重复初始化会继续报告故障；版本、描述查询和其他通道保持可用。调用 `CloseDevice` 清理故障通道，再调用 `InitDevice` 创建新对象；也可使用 `CloseAllDevices` 清理全部通道。

`CloseDevice` 无论关闭、析构或租约释放是否失败，都移除原通道并报告错误。`CloseAllDevices` 尝试全部通道后汇总清理错误。租约失效返回 `status = 2`，禁止继续访问设备，须先关闭再初始化。框架不会在故障对象上自动恢复业务调用。

FFI 和异步请求边界捕获额外运行时异常并执行清理。两个同步入口共用内部并发运行时，不再持有覆盖业务的全局锁；旧 Rust FFI 容器仅在取得共享运行时引用时短暂加锁，污染时先清理再恢复。`panic = "abort"`、内存访问违规与进程级崩溃无法由此机制恢复。

### 稳定响应代码

`MiresidianError::category()` 与 `code()` 是运行时错误分类的唯一来源；FFI 只把类别映射为数值状态。附加通道上下文不改变类别或代码。

| status | code | 含义 |
| --- | --- | --- |
| 0 | `OK` | 成功，业务结果位于 output。 |
| 1 | `INVALID_ARGUMENT` | UTF-8、JSON、参数类型、数量或配置不合法。 |
| 1 | `INVALID_OPERATION` | 未知操作。 |
| 1 | `INVALID_PLATFORM_ID` | 零值或未分配的保留平台编号。 |
| 1 | `UNKNOWN_PLATFORM_ADAPTER` | 平台未在节点 DLL 中注册。 |
| 2 | `NOT_INITIALIZED` | 尚未初始化。 |
| 2 | `UNKNOWN_CHANNEL` | 配置未对应已登记通道。 |
| 2 | `NOT_SUPPORTED` | 节点不支持该能力。 |
| 2 | `LEASE_INVALIDATED` | 租约失效，须关闭后重新初始化。 |
| 2 | `OPERATION_FAILED` | 业务执行或汇总清理失败。 |
| 3 | `NODE_PANICKED` | 节点发生已捕获的 panic。 |
| 3 | `NODE_STATE_POISONED` | 节点状态锁已污染。 |
| 3 | `INTERNAL_ERROR` | 其他框架异常或响应构造失败。 |

默认入口和显式平台入口使用同一响应模型；解析错误、panic 和兜底响应也包含四个字段。成功无输出时 output 为 `[]`，失败时 output 为 `[]`。两个版本查询分别从 output[0] 读取版本字符串。

增加 code 是兼容字段扩展：既有 status、message、output 的类型、导出函数、内存所有权和生命周期不变，C ABI 保持 v7。旧调用方可忽略新增字段，新调用方消费旧 DLL 时应允许 code 缺失；Windows TestFlow 执行器按此规则兼容并在提供 code 时显示它。

## 平台调用契约

本文描述 `tb_lv_test_platform` 的兼容调用约束。该平台规则由 `TbLvTestPlatformAdapter` 实现；运行时核心不依赖其 `共享` 标记或逗号协议。其他平台应通过 [平台适配器](#平台适配器) 实现各自映射。框架、属性宏、节点与发布文档的后续修改不得违背本文；若需要调整，必须先明确更新本文件并获得项目维护者确认。

### 调用模型与命名边界

节点业务调用模型为：

```text
Function + Config[] + Input[] -> Output[]
```

原始 C DLL 不使用该驼峰表示法作为导出符号。它固定导出 ABI 版本查询与两个业务调用入口：

| C ABI 入口 | 结果形式 |
| --- | --- |
| `miresidian_abi_version` | 返回当前 ABI 版本号。 |
| `miresidian_invoke` | 使用 InitDevice Config 选择平台，返回 DLL 持有的 `result_json`。 |
| `miresidian_invoke_with` | 显式传入已注册的平台枚举值，语义对应 `NodeRuntime::invoke_with`，返回相同 `result_json`。 |

`miresidian_invoke` 以 `function`、`config_json`、`input_json` 传入业务请求；`miresidian_invoke_with` 在其前额外传入 `uint32_t platform`。`config_json`
与 `input_json` 是 UTF-8 JSON 字符串数组；原生入口不会直接传递字符串指针数组。完整的参数
顺序、返回文本生命周期与状态码见 [C ABI](#c-abi)。

- `Function` 是平台调用入口中的函数名称。
- `Config[]` 与 `Input[]` 都是字符串数组，是相互独立的参数通道。
- `Output[]` 是有序字符串数组；无业务输出的方法返回空数组。
- Rust 运行时内部仍使用 `operation` 表示节点调度语义。不得将内部名称反向暴露为平台契约，也不得把平台的 `Function` 擅自改称为其他名称。

固定入口的外部名称为 `GetFunctions`、`GetFrameworkVersion`、`GetProjectVersion`、`GetInitConfig`、`GetPortNO`、`InitDevice`、`CloseDevice`、`CloseAllDevices`。其中两个版本查询不依赖设备初始化。普通节点方法使用 `operation` 标注发布名称。

### 参数通道

- `init_device` 的未标注参数默认来自 `Config[]`。
- 其他方法的未标注参数默认来自 `Input[]`。
- `#[miresidian::config(...)]` 与 `#[miresidian::input(...)]` 可标注在任意方法参数上，并覆盖自动来源；允许同一方法混用两个通道。
- 调用方必须按每个通道中参数的声明顺序传值。两个通道的索引彼此独立。

### 实例生命周期与多通道调用

完整 feature、宏配置、Config 布局和迁移规则见[实例管理](instances.md)。

- 默认节点只保留一个对象，不接受多通道请求，也不发布执行方式；同配置复用，异配置初始化先预校验，再释放旧对象并构造新对象。关闭或构造失败后不会继续使用已移除对象。
- 只有启用 `instances` feature 且声明同名宏配置才启用相应能力。multiple/config 以规范化业务 Config[0] 为主键；相同主键配置不一致报错。主键缺失或空白时初始化返回空 output 并跳过，不被默认值或其他参数补成新对象。
- multiple/handle 在业务 Config 后追加 Handle 项，相同 Handle 定位同一对象，不同 Handle 即使配置相同也不合并。空 Handle 初始化跳过；调用或关闭时为空报 INVALID_ARGUMENT，不存在报 UNKNOWN_CHANNEL。
- Config 顺序为业务项、可选 Handle、可选租约方式、可选平台、可选执行方式。框架字段统一剥离，不进入节点业务参数。普通入口发布平台选择器，显式入口不发布也不接受它。
- 固定 execution 不发布执行方式参数；selectable 才发布 TB 兼容的 `共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播`，缺省或空白采用声明默认值，未知文本兼容回退 Sequential。选择保存在每个对象上；相同主键重复初始化必须一致。
- 业务 panic 或租约失效后对象只能清理，其他对象、元数据与版本查询继续可用。CloseDevice 清理后始终移除对象；CloseAllDevices 不接受 Config/Input，逆序尝试清理全部对象并汇总错误。
- TB 自定义操作 Input[0] 为 `共享` 时映射成框架多通道请求。一份 Config 定位一次对象，各输入使用同一对象。后续输入项按英文逗号拆分参数，内部数组仍用 `|`。位置只对应输出位置，与创建顺序和对象数无关。
- Sequential 顺序执行，Parallel 调度 Future 但同一对象的完整调用仍互斥，Broadcast 固定使用通道1输入执行一次，将整份结果复制到有效通道，其他输入值不必相同。空白或参数数目非法的输入跳过并返回空字符串；实际执行失败时整个调用失败并保留平台通道序号，已完成的设备操作不会回滚。

### 参数描述文本原样透传

`config`、`input` 标注中的字符串是**平台参数描述文本**。属性宏必须原样发布给平台，不得解析、校验、重排、补充、翻译或改写其中的选项。

#### 描述文本书写规范

- 枚举项的实际值与显示值不同时，统一写为 `实际值-显示值`，例如
  `校验位:None-无校验/Odd-奇校验/Even-偶校验`；平台调用可传实际值 `Odd` 或显示值 `奇校验`。
- 实际值与显示值相同时只写一次，例如 `数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64`，不得写成
  `ASCII-ASCII` 一类重复文本。
- 仅有明确物理量单位的数值使用 `名称/单位:默认值`，例如 `读取超时/ms:1000`。计数、标识和
  普通数值不强行附加单位，例如 `接收字节数:0`、`波特率:9600`。

```rust
#[miresidian::config("校验位:None-无校验/Odd-奇校验/Even-偶校验")]
parity: SerialParity,
```

上述文本必须以完全相同的形式，作为 `GetFunctions` 或 `GetInitConfig` 的 Invoke 输出交给平台。例如：

```text
校验位:None-无校验/Odd-奇校验/Even-偶校验
```

框架原样发布描述，并在业务解析前将与某个 `实际值-显示值` 项精确匹配的显示值规范化为实际值；数组使用 `|` 时逐项规范化。完整的 `实际值-显示值` 描述文本不是业务输入。候选项的实际值和显示值唯一性由 DLL 项目保证。

### 描述查询与业务调用的两阶段规则

参数描述与参数值处于两个不同阶段，必须严格区分。

第一阶段，平台通过所选调用入口以 `GetFunctions` 或 `GetInitConfig` 查询描述。节点标注：

```rust
#[miresidian::config("校验位:None-无校验/Odd-奇校验/Even-偶校验")]
```

必须原样输出：

```text
校验位:None-无校验/Odd-奇校验/Even-偶校验
```

第二阶段，平台可选择 `Odd` 或 `奇校验` 后调用实际方法（例如 `InitDevice`）。框架会将后者规范化为前者；传入的参数值例如为：

```text
Odd
```

节点参数只解析 `Odd`，不会接收、也不得尝试解析完整描述文本 `None-无校验/Odd-奇校验/Even-偶校验`。描述文本仅在第一阶段用于平台查询。

### 参数值与节点解析

- 框架只执行显示值到实际值的文本规范化，不替代节点的 `DeviceInput` 类型校验。
- 平台向业务方法传入什么字符串、节点如何将其解析为 Rust 类型，仍属于平台与具体节点之间的既有契约。

### 数据格式与通用方法

数据格式是参数，不是方法名称。节点不得为了 ASCII、GBK、UTF-8、Unicode、HEX、Base64 等格式分别发布重复的发送、读取或查询方法。

通用字节通信节点应由“数据内容 + 数据格式”参数确定编解码行为；底层通信只处理原始字节。成功但无业务返回值的方法应返回空 `Output[]`，不得把底层写入字节数作为默认平台输出。

## 平台适配器

Miresidian 的运行时只负责节点生命周期、通道定位和结构化共享调度。具体测试平台的函数名、请求载荷、共享调用语法、错误码和响应编码由平台适配器负责。

### 命名

平台稳定标识使用小写 snake_case，例如当前兼容平台为 `tb_lv_test_platform`；对应 Rust 类型使用 PascalCase：`TbLvTestPlatformAdapter`。后续平台遵循同一规则。

### 接口

平台实现 `miresidian::PlatformAdapter`，并通过 `platform() -> Platform` 声明稳定平台枚举值；内置平台使用具名变体，第三方平台使用 `Platform::Custom(>= 0x0001_0000)`。适配器再将统一 C ABI 已解析出的 `Invocation` 映射为 `RuntimeInvocation`，并把 `RuntimeResponse` 编码为 `InvocationOutput`。适配器不应在业务节点或 `NodeRuntime` 中引入平台特有的文本协议。完全不同的原生传输或请求类型，应由宿主先转换为 `Invocation`，或直接构造 `RuntimeInvocation` 调用 `NodeRuntime::invoke_command`。

```text
Invocation
  -> PlatformAdapter::decode
  -> RuntimeInvocation
  -> NodeRuntime::invoke_command
  -> RuntimeResponse
  -> PlatformAdapter::encode
  -> InvocationOutput
```

### 平台选择

有两条互斥调用路径：

- `NodeRuntime::invoke(...)` 使用运行时注册表。`GetInitConfig` 与 `GetFunctions` 会发布保留 Config `平台适配:<id...>`；`InitDevice` 可选择其中一个已注册 id，选择不参与通道键，也不传给业务节点。运行时持有通道期间只能使用同一平台。
- `NodeRuntime::invoke_with(adapter, ...)` 由宿主显式指定适配器。此路径的初始化元数据不包含 `平台适配`，并会拒绝在 Config 中重复传入平台 id；其余 Config 仍按业务项、可选 Handle、可选租约及可选执行方式的布局解析。

默认构造的 `NodeRuntime::new()` 仅注册 `tb_lv_test_platform`。Rust 宿主可使用 `NodeRuntime::with_platforms(PlatformRegistry::new(...).with_adapter(...))` 注册更多适配器。

对 DLL 外部调用，适配器必须在节点声明时编译进 DLL：

```rust
#[miresidian::node("示例节点", platforms(MyPlatformAdapter))]
#[derive(Default)]
struct ExampleNode;

#[derive(Default)]
struct MyPlatformAdapter;
```

`platforms(...)` 接受一个或多个实现 `PlatformAdapter + Default` 的 Rust 类型路径；框架始终保留 `TbLvTestPlatformAdapter`，并在任一 C ABI 入口首次调用时用节点生成的注册表创建运行时。外部平台可继续用 `miresidian_invoke` 的 `InitDevice` Config 选择文本 id；需要显式选型时改用 `miresidian_invoke_with(uint32_t platform, function, config_json, input_json)`，它等价于 Rust 的 `invoke_with`，且初始化 Config 不包含平台选择器。

`RuntimeInvocation::MultiChannel` 使用 `MultiChannelInvocation { operation, config, slots }`，构造为 `MultiChannelInvocation::new(operation, config, slots)`。唯一 Config 按节点策略使用 Config[0] 或 Handle 定位一个对象，全部有效输入使用它。`MultiChannelResponse` 与输入保持相同顺序和数量；输入位置不是对象索引。

0.14.0 默认单实例；保留旧多对象行为需显式启用 feature 和 multiple/config 策略。旧 Rust 类型名 SharedInvocation/SharedResponse 保留别名，枚举变体改为 MultiChannel。C 入口、JSON 字符串数组编码、响应字段和内存约定不变，C ABI 保持 v7。

### 当前兼容平台

`TbLvTestPlatformAdapter` 保留既有约定：`Input[0] = "共享"` 触发共享调用，后续项以英文逗号拆分为单通道参数，结果以逗号拼接回平台字符串。该逗号约定仅存在于适配器中；未来 JSON 或其他平台适配器可以直接保留嵌套参数和多输出边界。

## 执行器与模块加载引用（0.15.1）

现有 miresidian_invoke / miresidian_invoke_with 保留同步返回方式，跨实例并行由运行时内部完成；C ABI 保持 v7，没有新增任务导出。
执行器启动后台线程前取得承载模块的额外加载引用；工作线程退出前该引用不会释放，避免宿主释放自身句柄导致后台代码悬空。
宿主每次调用及读取返回文本期间仍须持有自己的加载引用，不能在调用过程中释放最后一个宿主引用。
当前 C 入口没有最终执行器关闭操作，释放宿主句柄后模块可能保留至进程退出；Rust 嵌入可排空任务并关闭实例，再调用 shutdown_executor 等待线程退出和释放框架引用。
框架不使用不可逆 PIN/NODELETE；但其他引用（包括租约服务既有的 Windows PIN）仍可限制真实卸载。

## Broadcast 示例（0.16.0）

假设 Work 接收一个参数，实例 A 已选择 Broadcast：

```text
function = "Work"
config_json = ["A"]
input_json = ["共享", "10", "20", "", "30"]
```

仅执行通道1的 Work(10) 一次。若该次业务输出为 ["结果", "42"]，响应 output 为：

```json
["结果,42", "结果,42", "", "结果,42"]
```

通道1为空则全部槽位跳过；通道1解析或执行失败则整体返回错误，不选择后续通道。
跳过、零参数及迁移规则以[实例规范中的广播规则](instances.md#broadcast-的执行源与输出规则0160)为准。
这两个既有入口均遵循同一语义，函数签名和 C ABI v7 不变。
