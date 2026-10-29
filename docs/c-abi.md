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

FFI 在持有全局运行时锁的栈内捕获额外的运行时异常，清理全部通道后允许重新初始化；外部 Rust 调用者污染全局锁时也先清理再恢复。`panic = "abort"`、内存访问违规与进程级崩溃无法由此机制恢复。

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

### 通道生命周期与共享调用

- 普通 `NodeRuntime::invoke` 路径会在 `InitDevice` 的业务 Config（及可选租约方式）之后追加 `平台适配:<已注册 id...>`。该值仅用于从运行时注册表选择适配器，不参与通道键，也不传给节点；显式 `invoke_with(adapter, ...)` 路径不发布也不接受该项。宏生成的 C ABI 在首次调用时使用节点 `platforms(...)` 声明生成该注册表。
- 框架自动在 `InitDevice` 配置末尾追加 `共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播`。该值不传给节点 `init_device`，也不参与通道键；缺失、空白或未知值均回退为 `Sequential`。
- 首个成功初始化的通道确定本驱动运行时的共享执行方式；后续通道必须解析为同一方式。最后一个通道关闭后可重新选择。
- 节点业务操作发生可展开 panic 后，对应通道进入故障状态，禁止继续操作或复用；描述和版本查询、其他通道与清理入口继续可用。`CloseDevice` 尝试清理后始终移除对象，即使关闭、析构或租约释放失败；恢复须重新 `InitDevice`。
- 租约节点在初始化完成、复用通道及每次业务调用前校验原连接和服务代次。失联或超时后停止访问设备，关闭后才可重新初始化；存活客户端的配置文件锁在服务重启期间仍有效。
- `CloseAllDevices` 不接受 Config 或 Input；它按已创建通道的逆序逐一关闭、始终清空运行时登记并重置平台/共享执行状态。若任一关闭或租约释放失败，全部清理尝试完成后返回汇总错误。
- 除常驻查询和 `GetPortNO` 外，普通调用与 `CloseDevice` 以业务 Config 定位通道；运行时会补齐默认值、移除 `名称:` 前缀并将选项显示值规范化为实际值，携带的框架保留末项会自动剥离。
- 自定义操作的 `Input[0]` 为 `共享` 时进入共享调用。平台只传入一份 Config，框架以与普通调用相同的规则剥离保留项、规范化业务 Config 并定位唯一对象；全部平台通道输入共享该对象。随后每项为一个平台通道的 Input 参数串，使用英文逗号分隔参数；参数内部数组仍使用 `|` 语法。输入位置只用于输入输出对齐，不关联对象创建顺序，输入数量不受已初始化对象数限制。Output 与提供的输入保持相同顺序和数量；未知或已关闭 Config 返回 `UNKNOWN_CHANNEL`，不会改用其他对象。
- `Sequential` 按平台输入顺序在同一对象上执行；`Parallel` 为有效输入创建线程，但同一对象的完整方法调用仍由同一把锁互斥保护，不保证输入开始顺序；`Broadcast` 要求有效输入参数串完全一致，再按平台输入顺序在同一对象上逐项执行，不合并业务调用。
- 空白通道项、空参数项、参数过多或缺少无默认值参数的通道均跳过，且在对应 Output 槽位返回空字符串。共享调用中任一实际执行失败时，整个调用失败并指出通道序号。

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
- `NodeRuntime::invoke_with(adapter, ...)` 由宿主显式指定适配器。此路径的初始化元数据不包含 `平台适配`，并会拒绝在 Config 中重复传入平台 id；只保留业务 Config、可选租约方式与共享执行方式。

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

`RuntimeInvocation::Shared` 使用 `SharedInvocation { operation, config, slots }`，通过 `SharedInvocation::new(operation, config, slots)` 构造。一份 Config 定位唯一节点对象，所有有效输入使用该对象；槽位表示平台通道数据位置，不表示对象索引，数量与已创建对象数无关。`SharedResponse` 与请求槽位保持相同顺序和数量。

0.13.0 修正了旧版按创建顺序选择多个对象的错误行为。迁移时，共享请求应传入目标对象的业务 Config；需要访问不同 Config 对应对象时，应分别调用。公开 Rust 构造函数新增 Config 参数；C 入口、JSON 字符串数组编码、响应字段和内存约定不变，C ABI 保持 v7。

### 当前兼容平台

`TbLvTestPlatformAdapter` 保留既有约定：`Input[0] = "共享"` 触发共享调用，后续项以英文逗号拆分为单通道参数，结果以逗号拼接回平台字符串。该逗号约定仅存在于适配器中；未来 JSON 或其他平台适配器可以直接保留嵌套参数和多输出边界。
