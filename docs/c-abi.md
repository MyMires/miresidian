# C ABI

Miresidian 节点 DLL 导出默认入口 `miresidian_invoke` 与显式平台入口 `miresidian_invoke_with`。两个入口均使用 UTF-8 JSON 文本，
不要求调用方分配输出缓冲区，不传递 Rust 结构体。

ABI 版本 miresidian_abi_version() 当前返回 7。

## 入口声明

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

## 响应体

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

## 状态码

| 值 | 含义 |
| --- | --- |
| 0 | 成功 |
| 1 | 参数无效：ABI 文本/JSON 不合法，或运行时拒绝参数、运行时明确报告的未知操作。 |
| 2 | 操作失败：未初始化、通道不存在、节点关闭或业务执行失败。 |
| 3 | 内部错误 |

## 异常隔离与恢复

节点的可展开 panic 返回 `status = 3`。业务操作 panic 后对应通道被标记为故障，后续操作和重复初始化会继续报告故障；版本、描述查询和其他通道保持可用。调用 `CloseDevice` 清理故障通道，再调用 `InitDevice` 创建新对象；也可使用 `CloseAllDevices` 清理全部通道。

`CloseDevice` 无论关闭、析构或租约释放是否失败，都移除原通道并报告错误。`CloseAllDevices` 尝试全部通道后汇总清理错误。租约失效返回 `status = 2`，禁止继续访问设备，须先关闭再初始化。框架不会在故障对象上自动恢复业务调用。

FFI 在持有全局运行时锁的栈内捕获额外的运行时异常，清理全部通道后允许重新初始化；外部 Rust 调用者污染全局锁时也先清理再恢复。`panic = "abort"`、内存访问违规与进程级崩溃无法由此机制恢复。

## 稳定响应代码

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
