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

返回文本始终是同一种 JSON 对象，字段顺序固定为 status、message、output。

成功时：

~~~json
{
  "status": 0,
  "message": "",
  "output": ["业务输出1", "业务输出2"]
}
~~~

失败时：

~~~json
{
  "status": 2,
  "message": "设备尚未初始化",
  "output": []
}
~~~

status 为状态码，message 为成功时的空字符串或失败诊断，output 始终为业务字符串数组。
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
