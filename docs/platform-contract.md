# TB LV Test Platform 调用契约

本文描述 `tb_lv_test_platform` 的兼容调用约束。该平台规则由 `TbLvTestPlatformAdapter` 实现；运行时核心不依赖其 `共享` 标记或逗号协议。其他平台应通过 [平台适配器](platform-adapters.md) 实现各自映射。框架、属性宏、节点与发布文档的后续修改不得违背本文；若需要调整，必须先明确更新本文件并获得项目维护者确认。

## 调用模型与命名边界

节点业务调用模型为：

```text
Function + Config[] + Input[] -> Output[]
```

原始 C DLL 不使用该驼峰表示法作为导出符号。它固定导出 ABI 版本查询与唯一业务入口：

| C ABI 入口 | 结果形式 |
| --- | --- |
| `miresidian_abi_version` | 返回当前 ABI 版本号。 |
| `miresidian_invoke` | 返回 DLL 持有的 `result_json`，固定为 `{ "status": 状态码, "message": 消息, "output": [...] }`。 |

`miresidian_invoke` 以 `function`、`config_json`、`input_json` 传入业务请求。`config_json`
与 `input_json` 是 UTF-8 JSON 字符串数组；原生入口不会直接传递字符串指针数组。完整的参数
顺序、返回文本生命周期与状态码见 [C ABI](c-abi.md)。

- `Function` 是平台调用入口中的函数名称。
- `Config[]` 与 `Input[]` 都是字符串数组，是相互独立的参数通道。
- `Output[]` 是有序字符串数组；无业务输出的方法返回空数组。
- Rust 运行时内部仍使用 `operation` 表示节点调度语义。不得将内部名称反向暴露为平台契约，也不得把平台的 `Function` 擅自改称为其他名称。

固定入口的外部名称为 `GetFunctions`、`GetFrameworkVersion`、`GetProjectVersion`、`GetInitConfig`、`GetPortNO`、`InitDevice`、`CloseDevice`。其中两个版本查询不依赖设备初始化。普通节点方法使用 `operation` 标注发布名称。

## 参数通道

- `init_device` 的未标注参数默认来自 `Config[]`。
- 其他方法的未标注参数默认来自 `Input[]`。
- `#[miresidian::config(...)]` 与 `#[miresidian::input(...)]` 可标注在任意方法参数上，并覆盖自动来源；允许同一方法混用两个通道。
- 调用方必须按每个通道中参数的声明顺序传值。两个通道的索引彼此独立。

## 通道生命周期与共享调用

- 普通 `NodeRuntime::invoke` 路径会在 `InitDevice` 的业务 Config（及可选租约方式）之后追加 `平台适配:<已注册 id...>`。该值仅用于从运行时注册表选择适配器，不参与通道键，也不传给节点；显式 `invoke_with(adapter, ...)` 路径不发布也不接受该项。宏生成的 C ABI 在首次调用时使用节点 `platforms(...)` 声明生成该注册表。
- 框架自动在 `InitDevice` 配置末尾追加 `共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播`。该值不传给节点 `init_device`，也不参与通道键；缺失、空白或未知值均回退为 `Sequential`。
- 首个成功初始化的通道确定本驱动运行时的共享执行方式；后续通道必须解析为同一方式。最后一个通道关闭后可重新选择。
- 除常驻查询和 `GetPortNO` 外，普通调用与 `CloseDevice` 以业务 Config 定位通道；运行时会补齐默认值、移除 `名称:` 前缀并将选项显示值规范化为实际值，携带的框架保留末项会自动剥离。
- 自定义操作的 `Input[0]` 为 `共享` 时进入共享调用。随后每项依创建通道顺序对应一个通道，使用英文逗号分隔该通道的 Input 参数；参数内部数组仍使用 `|` 语法，输出也一通道一个槽位。
- `Sequential` 顺序执行，`Parallel` 并行执行，`Broadcast` 要求各有效通道参数串完全一致，并以同一参数计划顺序执行到各目标通道。
- 空白通道项、空参数项、参数过多或缺少无默认值参数的通道均跳过，且在对应 Output 槽位返回空字符串。共享调用中任一实际执行失败时，整个调用失败并指出通道序号。

## 参数描述文本原样透传

`config`、`input` 标注中的字符串是**平台参数描述文本**。属性宏必须原样发布给平台，不得解析、校验、重排、补充、翻译或改写其中的选项。

### 描述文本书写规范

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

## 描述查询与业务调用的两阶段规则

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

## 参数值与节点解析

- 框架只执行显示值到实际值的文本规范化，不替代节点的 `DeviceInput` 类型校验。
- 平台向业务方法传入什么字符串、节点如何将其解析为 Rust 类型，仍属于平台与具体节点之间的既有契约。

## 数据格式与通用方法

数据格式是参数，不是方法名称。节点不得为了 ASCII、GBK、UTF-8、Unicode、HEX、Base64 等格式分别发布重复的发送、读取或查询方法。

通用字节通信节点应由“数据内容 + 数据格式”参数确定编解码行为；底层通信只处理原始字节。成功但无业务返回值的方法应返回空 `Output[]`，不得把底层写入字节数作为默认平台输出。
