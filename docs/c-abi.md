# 平台接入与 C ABI

[文档中心](README.md) · [架构总览](architecture.md)

本文集中说明节点 DLL 的跨语言入口、TB LV Test Platform 调用契约和平台适配扩展。宿主集成先读 C ABI，平台调用规则见第二章，新增适配器见第三章。

**章节导航：[C ABI](#c-abi) · [平台调用契约](#平台调用契约) · [平台适配器](#平台适配器)**

## C ABI

Miresidian 节点 DLL 导出完整请求入口 `miresidian_invoke_request`，并保留默认入口 `miresidian_invoke` 与显式平台入口 `miresidian_invoke_with`。三个入口均使用 UTF-8 文本，
不要求调用方分配输出缓冲区，不传递 Rust 结构体。

ABI 版本由 miresidian_abi_version() 查询；本页描述 C ABI v8，签名以[公共头文件](../abi/api/miresidian.h)为准。组件版本和发行状态按[实际版本](README.md#按实际版本查阅)核对。

### 入口声明

~~~c
uint32_t miresidian_abi_version(void);

const char *miresidian_invoke_request(const char *request_json);

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

request_json 为完整请求入口的单个 UTF-8 JSON 对象文本，契约见下一节；其余三个参数仅属于既有入口。

三个调用入口共享运行时及当前线程响应缓存。DLL 在当前线程中持有返回文本。调用方无需分配输出缓冲区、无需传容量、也不得释放返回指针；
应在同一线程下一次调用任一调用入口前将文本复制到自己的内存；线程退出或 DLL 卸载同样使指针失效，以最早发生者为准。ABI 版本查询不替换响应缓存。

默认入口按 `InitDevice` 的保留 `平台适配` Config 选择平台。显式平台入口对应 Rust 的 `NodeRuntime::invoke_with`：平台枚举数值单独传入，`InitDevice` Config 不发布也不接受 `平台适配`。

例如，调用 GetFunctions：

~~~c
const char *result_json = miresidian_invoke(
    "GetFunctions", "[]", "[]");
~~~

### 完整请求入口（v8）

新入口将一次调用整体序列化，适合宿主统一保存、传递和回放请求：

~~~c
const char *result_json = miresidian_invoke_request(
    "{\"function\":\"GetFunctions\",\"config\":[],\"input\":[]}");
/* 立即复制 result_json；不得修改或释放 DLL 返回指针。 */
~~~

~~~json
{
  "platform": 1,
  "function": "GetFunctions",
  "config": [],
  "input": []
}
~~~

| 字段 | 类型与默认 | 规则 |
| --- | --- | --- |
| function | 必填字符串 | 非空，操作名精确匹配，不 trim |
| config | 字符串数组，省略为 [] | 按 Config 声明顺序；不可把数组再次编码为字符串 |
| input | 字符串数组，省略为 [] | 按 Input 声明顺序 |
| platform | 可选 u32 JSON 整数 | 省略沿用 miresidian_invoke；提供沿用 miresidian_invoke_with |

字段顺序任意；拒绝未知字段、重复字段、任何字段的显式 null、非对象请求、非法 JSON/UTF-8、空指针及错误类型，返回 `status:1/code:INVALID_ARGUMENT/output:[]`。platform 不接受名称、数字字符串（如 `"1"`）、布尔、负数、小数（包括 `1.0`/`1e0`）或整数溢出。

平台编号是稳定标识，不是 Rust/C 枚举声明顺序：`1` 对应 `MIRESIDIAN_PLATFORM_TB_LV_TEST_PLATFORM`，`2..65535` 当前保留，第三方从 `MIRESIDIAN_PLATFORM_CUSTOM_MIN`（65536）开始且须编译并注册进当前 DLL。`0` 或无效保留编号返回 `INVALID_PLATFORM_ID`；合法未注册编号（包括未注册的 u32 最大值）返回 `UNKNOWN_PLATFORM_ADAPTER`，status 均为 1。升级不能随意重新分配既有编号。

省略 platform 不等于补 1：初始化前查询、`InitDevice` 的“平台适配”保留 Config、选型后的调用及平台切换限制均沿用默认入口。例如先查询 `{"function":"GetInitConfig"}`，按发布顺序传业务 Config 与所选平台文本，再初始化。后续省略 platform 的请求继续使用运行时已选择的平台。显式提供 platform 时初始化元数据和 Config 均不含该选择器，重复传入返回参数错误。

request_json 非 NULL 时必须是调用期间可读、NUL 结尾的 C 字符串；输入内存归调用方。无效地址或缺少 NUL 终止符无法安全校验。可展开 panic 按既有错误隔离规则返回统一响应；新入口不新增线程、设备实例或独立响应缓存。

#### v7 到 v8 迁移

v8 保留两个既有调用入口的签名和行为，响应字段与内存所有权不变。可继续使用旧入口；需要完整请求时先确认 DLL 报告且支持 ABI v8，再解析新导出符号。只有 v7 的旧 DLL 不提供此符号；v8 也不代表以后所有更高 ABI 都自动兼容，宿主应维护经过评估的支持版本集合。原先要求版本恰等于 7 的宿主需要显式增加 v8 支持。

各版本的引入与发行记录见[版本归档](releases/0.26.0.md)；查阅或采用其他版本时按[实际版本](README.md#按实际版本查阅)核对，不从当前手册推断旧 DLL 的能力。

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
只有节点定义的业务操作才会在未初始化时返回“设备尚未初始化”。

### 固定查询与发现结果

`GetFunctions`、`GetInitConfig`、版本查询与 `GetPortNO` 均可在初始化前执行。`GetPortNO` 的 Input 对应节点声明的探测参数，不能把 Config 中的已知地址直接当作探测成功。

| 节点探测结果 | 平台响应 |
| --- | --- |
| `Ok(Some(address))` | 成功，output 为 `["1", "地址"]`；地址格式由节点声明 |
| `Ok(None)` 或未实现探测 | 成功，output 为 `["0", ""]`，表示正常执行但未找到设备 |
| `Err(error)` | 按统一错误响应返回非成功 status/code 和空 output；不得伪装为未找到 |

两个版本查询分别在 output[0] 返回框架和节点版本。初始化、关闭、发现的 Rust 签名及资源职责统一见 [固定入口契约](宏驱动开发.md#固定入口的实现与失败边界)。

### 状态码

| 值 | 含义 |
| --- | --- |
| 0 | 成功 |
| 1 | 参数无效：ABI 文本/JSON 不合法，或运行时拒绝参数、运行时明确报告的未知操作。 |
| 2 | 操作失败：未初始化、通道不存在、节点关闭或业务执行失败。 |
| 3 | 内部错误 |

### 异常隔离与恢复

节点的可展开 panic 返回 `status = 3`。业务操作 panic 后对应通道被标记为故障，后续操作和重复初始化会继续报告故障；版本、描述查询和其他通道保持可用。调用 `CloseDevice` 清理故障通道，再调用 `InitDevice` 创建新对象；也可使用 `CloseAllDevices` 清理全部通道。

`CloseDevice` 无论关闭、析构或租约释放是否失败，都移除原通道并报告错误。`CloseAllDevices` 尝试全部通道后汇总清理错误。租约失效返回 `status = 2`，禁止继续访问目标资源，须先关闭再初始化。框架不会在故障对象上自动恢复业务调用。

FFI 和异步请求边界捕获额外运行时异常并执行清理。三个同步入口共用内部并发运行时，不再持有覆盖业务的全局锁；旧 Rust FFI 容器仅在取得共享运行时引用时短暂加锁，污染时先清理再恢复。`panic = "abort"`、内存访问违规与进程级崩溃无法由此机制恢复。

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

三个调用入口使用同一响应模型；解析错误、panic 和兜底响应也包含四个字段。成功无输出时 output 为 `[]`，失败时 output 为 `[]`。两个版本查询分别从 output[0] 读取版本字符串。

在历史 v7 中增加 code 是兼容字段扩展：既有 status、message、output 的类型、导出函数、内存所有权和生命周期不变，当时 C ABI 保持 v7。旧调用方可忽略新增字段，新调用方消费旧 DLL 时应允许 code 缺失；Windows TestFlow 执行器按此规则兼容并在提供 code 时显示它。

## 平台调用契约

本文描述 `tb_lv_test_platform` 的兼容调用约束。该平台规则由 `TbLvTestPlatformAdapter` 实现；运行时核心不依赖其 `共享` 标记或逗号协议。其他平台应通过 [平台适配器](#平台适配器) 实现各自映射。框架、属性宏、节点与发布文档的后续修改不得违背本文；若需要调整，必须先明确更新本文件并获得项目维护者确认。

### 调用模型与命名边界

节点业务调用模型为：

```text
Function + Config[] + Input[] -> Output[]
```

原始 C DLL 不使用该驼峰表示法作为导出符号。它固定导出 ABI 版本查询与三个业务调用入口：

| C ABI 入口 | 结果形式 |
| --- | --- |
| `miresidian_abi_version` | 返回当前 ABI 版本号。 |
| `miresidian_invoke_request` | 接收完整请求对象，可选数值 platform；省略时由 InitDevice Config 选择平台。 |
| `miresidian_invoke` | 使用 InitDevice Config 选择平台，返回 DLL 持有的 `result_json`。 |
| `miresidian_invoke_with` | 显式传入已注册的平台枚举值，语义对应 `NodeRuntime::invoke_with`，返回相同 `result_json`。 |

`miresidian_invoke_request` 使用上一节的完整 JSON 请求对象。既有 `miresidian_invoke` 以 `function`、`config_json`、`input_json` 传入业务请求；`miresidian_invoke_with` 在其前额外传入 `uint32_t platform`。`config_json`
与 `input_json` 是 UTF-8 JSON 字符串数组；原生入口不会直接传递字符串指针数组。完整的参数
顺序、返回文本生命周期与状态码见 [C ABI](#c-abi)。

- `Function` 是平台调用入口中的函数名称。
- `Config[]` 与 `Input[]` 都是字符串数组，是相互独立的参数通道。
- `Output[]` 是有序字符串数组；无业务输出的方法返回空数组。
- Rust 运行时内部仍使用 `operation` 表示节点调度语义。不得将内部名称反向暴露为平台契约，也不得把平台的 `Function` 擅自改称为其他名称。

固定入口的外部名称为 `GetFunctions`、`GetFrameworkVersion`、`GetProjectVersion`、`GetInitConfig`、`GetPortNO`、`InitDevice`、`CloseDevice`、`CloseAllDevices`。其中两个版本查询不依赖节点初始化。普通节点方法使用 `operation` 标注发布名称。

### 参数通道

Config 与 Input 是两个独立的有序字符串数组，调用方按 GetInitConfig/GetFunctions 查询到的布局分别传入。节点作者的默认来源和逐参数标注语法见[节点标注规则](宏驱动开发.md#标注规则)。

### 实例生命周期与多通道调用

对象复用、关闭和执行策略见[实例管理](instances.md)，实例键选择见[Config 身份](config-identity.md)，框架追加字段见[Config 布局](instances.md#框架-config-布局)。宿主调用前应查询元数据，不在本页维护另一份对象规则。

CloseAllDevices 不接受 Config/Input，逆序尝试清理全部对象并汇总错误。TB 平台自定义操作 Input[0] 为 `共享` 时映射成多通道请求：一份 Config 定位一个对象，后续输入项按英文逗号拆分参数，内部数组仍用 `|`。位置对应输出位置，与对象创建顺序无关。空白或参数数目非法的输入跳过，输出位置保留空字符串；实际执行失败时整个调用失败并保留平台通道序号，已完成的业务不会回滚。执行次数与顺序按[多通道规则](instances.md#多通道执行)。

### 参数描述文本原样透传

`config`、`input` 标注中的字符串是**平台参数描述文本**。属性宏必须原样发布给平台，不得解析、校验、重排、补充、翻译或改写其中的选项。

#### 描述文本书写规范

节点作者的描述、预填、选项和单位书写统一见[参数声明规范](宏驱动开发.md#参数声明规范)。平台负责原样展示元数据，并在调用阶段提交实际值；两阶段示例如下。

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

节点的 DeviceInput 接收显示值映射后的实际值。调用方须传入具体值；完整候选文本 `9600/19200` 不自动变成 9600，数值类型报错，String 保留原文。缺参报错，显式数值空白为零；平台预填内容属于实际输入，不是框架默认值。

### 参数值与节点解析

- 预填内容属于宿主实际输入；声明约束见[参数声明规范](宏驱动开发.md#参数声明规范)。
- 框架统一处理参数名前缀及具体选项显示值映射，再交给 DeviceInput 转换类型；缺失/空值区别见[强类型参数](宏驱动开发.md#强类型参数)。
- 候选列表不自动限制合法集合：例如 u32 可解析列表外的 `38400`；节点须在资源操作前校验真实允许范围，不能把非法业务值回退到首项。

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

`platforms(...)` 接受一个或多个实现 `PlatformAdapter + Default` 的 Rust 类型路径；框架始终保留 `TbLvTestPlatformAdapter`，并在任一 C ABI 入口首次调用时用节点生成的注册表创建运行时。外部平台可继续用 `miresidian_invoke` 的 `InitDevice` Config 选择文本 id；需要显式选型时改用 `miresidian_invoke_with(uint32_t platform, function, config_json, input_json)`，它等价于 Rust 的 `invoke_with`，且初始化 Config 不包含平台选择器。完整请求入口按 platform 字段是否提供分别复用这两条路径。

`RuntimeInvocation::MultiChannel` 使用 `MultiChannelInvocation { operation, config, slots }`，构造为 `MultiChannelInvocation::new(operation, config, slots)`。唯一 Config 按节点策略使用 Config 组合键或 Handle 定位一个对象，全部有效输入使用它。`MultiChannelResponse` 与输入保持相同顺序和数量；输入位置不是对象索引。

0.14.0 默认单实例；保留旧多对象行为需显式启用 feature 和 multiple/config 策略。旧 Rust 类型名 SharedInvocation/SharedResponse 保留别名，枚举变体改为 MultiChannel。C 入口、JSON 字符串数组编码、响应字段和内存约定不变，C ABI 保持 v7。

### 当前兼容平台

`TbLvTestPlatformAdapter` 保留既有约定：`Input[0] = "共享"` 触发共享调用，后续项以英文逗号拆分为单通道参数，结果以逗号拼接回平台字符串。该逗号约定仅存在于适配器中；未来 JSON 或其他平台适配器可以直接保留嵌套参数和多输出边界。

## 执行器与模块加载引用（0.15.1）

三个调用入口都同步返回，跨实例并行由运行时内部完成；当前 C ABI v8 只新增完整请求入口，没有新增任务导出。
执行器启动后台线程前取得承载模块的额外加载引用；工作线程退出前该引用不会释放，避免宿主释放自身句柄导致后台代码悬空。
宿主每次调用及读取返回文本期间仍须持有自己的加载引用，不能在调用过程中释放最后一个宿主引用。
当前 C 入口没有最终执行器关闭操作，释放宿主句柄后模块可能保留至进程退出；Rust 嵌入可排空任务并关闭实例，再调用 shutdown_executor 等待线程退出和释放框架引用。
框架不使用不可逆 PIN/NODELETE；但其他引用（包括租约服务既有的 Windows PIN）仍可限制真实卸载。

## Broadcast 示例（0.16.0）

执行次数、源通道、输出位置及完整示例统一见[广播规范](instances.md#broadcast-的执行源与输出规则0160)；TB 的外层字符串拆分与拼接按[当前兼容平台](#当前兼容平台)处理。

## batch 与格式选择输入

batch 方法的业务参数仍按 GetFunctions 发布的顺序组织 Input 字符串数组。普通 String/bool/数值 batch 使用原类型的列表语法；整数支持 `1-3|5|8-10` 混合展开，字符串不将范围样文本展开。

标记 batch 且有多种 format 的数据参数，其紧随的格式选择输入支持单值广播或等长列表。例如数据 `A|42|Qw==`、格式 `UTF8|HEX|Base64` 解码为三条 String。batch 方法的输出格式选择输入也支持单值或列表，位置仍在所有业务 Input 后；无 batch 时格式选择仍为单值。

只有一种编码的 format 不生成选择输入：固定 HEX 的 String batch 只传 `41|42` 一个 Input。数据与格式列解析、类型校验和对齐均在业务执行前完成；输出转换失败则可能已发生业务副作用，不回滚。输出仍是原有字符串数组，各槽位按调用顺序聚合，不新增 C 导出入口或改变 ABI v7。

### 0.25.0 Lease 引用生命周期

共享初始化、最后引用关闭和迁移统一见[共享资源](shared-resources.md#现有-lease-标签自动共享节点资源)，不新增 C ABI 入口；宿主仍按本页的同步响应契约调用。

### 当前完整请求 ABI 的适用范围（0.26.0）

本页历史特性段落中的“ABI v7 不变”指各特性引入当时的既有两个入口。当前 DLL 版本为 v8，新增完整请求入口；业务 Config/Input 布局、batch/format、同步调度、错误响应和租约/资源协议均未由本次改变。

模板接受的 ABI 范围按所用版本的 [Pack 手册](../tools/miresidian-pack/README.md#最小模板)核对；TestFlow 的实际调用入口按其[工具说明](../tools/miresidian-testflow-windows/README.md)核对，不从框架版本推断工具能力。
