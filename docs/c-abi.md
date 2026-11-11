# 平台接入与 C ABI

[文档中心](README.md) · [节点开发](宏驱动开发.md) · [架构总览](architecture.md)

本页描述本检出 0.30.0 源码候选的 ABI v10；候选尚未发布。旧版按实际标签/发行包查阅，不把本页签名用于旧 DLL。[迁移说明](releases/0.30.0.md)记录 v9 到 v10 的不兼容变更。

## C ABI

### 入口声明

头文件为 [miresidian.h](../abi/api/miresidian.h)。唯一业务调用入口接受完整 JSON：

```c
uint32_t miresidian_abi_version(void);
const char *miresidian_invoke(const char *request_json);
```

Windows 使用 Cdecl。宿主必须先调用版本查询并确认 ABI **10**，再按一个字符串参数绑定 miresidian_invoke；v7-v9 的同名入口是三个参数，不能混用函数指针。v10 不导出 miresidian_invoke_with 或 miresidian_invoke_request，也不提供旧签名别名。版本查询用于调用前校验，不替换业务响应缓存。

### 唯一 JSON 请求（v10）

```json
{"platform":1,"function":"GetFunctions","config":[],"input":[]}
```

| 字段 | 类型与缺省 | 约束 |
| --- | --- | --- |
| function | 必填字符串 | 非空，精确匹配操作名，不 trim |
| config | 字符串数组，缺省 [] | 按 Config 声明顺序 |
| input | 字符串数组，缺省 [] | 按 Input 声明顺序 |
| platform | u32 JSON 整数，缺省 0 | 0 使用 DLL 默认平台，非零指定已注册平台 |

字段顺序任意。拒绝非对象、未知字段、重复字段、显式 null、数组内非字符串及错误类型；数组不能再次编码为一个 JSON 字符串。platform 不接受名称、数字字符串、布尔、负数（含 -0）、小数（含 1.0/1e0）或溢出。

平台在每次 DLL 调用时确定，不再让节点用户通过 Config 选择：

- `0` 或省略：使用注册表固定默认平台，通常为内置 TB；不沿用之前显式自定义平台。
- `1`：内置 TB；`2..65535` 保留；自定义编号从 `65536` 开始，必须已编译并注册进 DLL。
- 无效保留编号返回 INVALID_PLATFORM_ID，合法未注册编号返回 UNKNOWN_PLATFORM_ADAPTER。
- GetFunctions/GetInitConfig 不发布“平台适配”Config，InitDevice 传入旧平台选择项返回 INVALID_ARGUMENT。
- 实例存活期间本次平台必须与绑定平台一致；关闭全部实例后可重新显式指定。默认平台和显式平台调用使用同一运行时，不创建平行状态。

```json
{"platform":1,"function":"InitDevice","config":["COM3"],"input":[]}
```

实际 Config/Input 布局必须按当前 DLL 元数据查询，不照抄其他节点示例。业务 Config 后依声明追加 Handle、可选资源访问方式、可选共享执行方式；没有平台选择槽位。

### 响应体

```json
{"status":0,"code":"OK","message":"","output":["结果"]}
```

字段顺序固定为 status、code、message、output。output 恒为有序字符串数组；无业务输出或失败时为 []。空操作返回空数组，不默认把写入字节数等底层信息作为业务结果。

| status | 说明 |
| --- | --- |
| 0 | 成功 |
| 1 | 请求、参数、类型或平台编号错误 |
| 2 | 生命周期、业务执行或不支持能力等预期失败 |
| 3 | 内部异常或已隔离的 panic |

code 是稳定机器代码，宿主按 status/code 分类，不解析中文 message。常见代码包括 OK、INVALID_ARGUMENT、INVALID_PLATFORM_ID、UNKNOWN_PLATFORM_ADAPTER、NOT_INITIALIZED、UNKNOWN_CHANNEL、INVALID_OPERATION、NOT_SUPPORTED、NODE_PANICKED、INTERNAL_ERROR、LEASE_INVALIDATED；实际可返回集合以 [错误定义](../codec/src/error.rs)为准，不把诊断文字当作协议。

### 指针、缓存与所有权

request_json 非 NULL 时，调用方保证它是本次调用期间可读、NUL 结尾的 UTF-8 C 字符串；输入内存仍归调用方。NULL、非法 UTF-8/JSON 可通过响应报告；无效地址和缺少 NUL 无法安全校验。JSON 的字符串转义可以承载逻辑 NUL，不能直接在请求 C 字符串中插入终止字节。

DLL 持有返回指针，宿主必须立即复制，不得修改或释放。同线程下一次 miresidian_invoke、线程退出或 DLL 卸载使上一份指针失效，以最早发生者为准。跨调用、跨线程或异步保存结果时使用宿主自己的副本；版本查询不替换响应缓存。DLL 卸载前必须停止并发入口并按[运行时执行约束](architecture.md#并行与异步执行)回收调用及执行器。

### 固定查询与发现结果

固定操作为 GetFunctions、GetFrameworkVersion、GetProjectVersion、GetInitConfig、GetPortNO、InitDevice、CloseDevice、CloseAllDevices。

版本、元数据和 GetPortNO 不要求已初始化；CloseAllDevices 幂等地尝试关闭全部对象。GetPortNO 找到时输出 ["1", "端口号"]，正常未找到或未实现时为 ["0", ""]；执行失败返回错误，不伪装成未找到。框架版本与项目版本分别查询，不用 ABI 版本替代语义版本。

### 异常隔离与恢复

业务操作 panic 后对应对象被标记为故障；其他通道和常驻查询保持可用。CloseDevice 尽力关闭并移除目标对象，即使关闭、析构或租约释放失败；CloseAllDevices 尝试所有对象并汇总错误。租约失效后禁止继续访问资源，需关闭再初始化。具体所有权与清理顺序见[实例管理](instances.md)和[共享资源](shared-resources.md)。

### 完整请求入口（v8）

这是历史查阅入口。v8 的 miresidian_invoke_request 和 v9 的 Config 平台选择仅在对应旧版本存在；当前 v10 将单请求入口更名为 miresidian_invoke 并移除分参数入口。旧调用说明见[0.26.0](releases/0.26.0.md)、[0.27.0](releases/0.27.0.md)，升级按[0.30.0](releases/0.30.0.md)。

## 平台调用契约

### 调用模型与命名边界

请求采用 Function + Config + Input，响应采用 status/code/message/output。操作名由 node/operation 宏生成；业务节点不手写 GetFunctions/GetInitConfig、字符串分发或 C ABI 导出。Rust 生命周期 init_device/close_device 仍对应既有 InitDevice/CloseDevice，本次仅收敛 C 调用入口。

Config 与 Input 是两个独立、有序的字符串数组；名称前缀不触发按名称重排。InitDevice 的函数描述按 Config 后接初始化 Input 发布；调用时仍分属两个数组。

### 参数描述文本原样透传

手写非空默认/候选描述是显式平台契约，仍原样发布，不补充、重排、翻译或改写。例如：

```rust,ignore
#[miresidian::input("状态:Idle-空闲/Running-运行中")]
state: State,
```

TB 平台接收 Idle 或空闲时先映射为实际值 Idle，再由 codec 转为 State::Idle。完整候选文本及 Idle-空闲 是描述文本，不是具体输入。候选列表不自动约束普通 u32/String 等类型的合法业务集合。

### 自动枚举候选与平台适配

无字段业务枚举可使用 #[derive(miresidian::NodeEnum)]，codec 提供结构化 EnumOption 候选。参数描述没有非空冒号后缀时自动使用这些候选；已手写非空默认/候选时保留原文。自动候选不是运行时缺参默认值，不隐式选首项。

格式由本次请求确定的平台负责：

| 层 | 职责 |
| --- | --- |
| 枚举派生 / codec | 实际值与候选元数据、纯 Rust 类型转换、集合及可选值规则 |
| 参数宏 | 将原 Rust 类型的候选绑定到对应参数 |
| PlatformAdapter::format_parameter | 发布本平台候选格式 |
| PlatformAdapter::normalize_parameter | 把同一平台的枚举输入转为 codec 实际值或标准集合文本 |
| decode / encode | 平台传输表示与结构化运行时命令/响应转换 |

TB 自动生成 名称:A/B 或 名称:A-显示值/B-显示值；其他适配器可以生成 JSON 等表示，配套接收自己的输入格式。自动 TB 候选拒绝其保留分隔符与歧义标签，这些限制不施加到通用 EnumOption 或其他平台。未实现枚举支持的平台明确返回 NOT_SUPPORTED，不能默用 TB 规则。

元数据和输入规范化使用同一个真实适配器对象，每次查询独立渲染，不缓存第一次平台的候选字符串。Config、初始化/探测/业务 Input、多通道槽位使用对应参数元数据；普通类型及 format 转换参数沿用既有规则。format 的业务类型不被额外要求实现 NodeInput。

### 参数值与节点解析

缺参报错；数值显式空白为零，Option 显式空白为 None，Vec 显式空白为空集合。节点仍校验业务零值是否有效。路径保留原文，不自动转换分隔符。完整候选文本不自动取首项，枚举未知值返回 INVALID_ARGUMENT。完整类型和数组规则见[节点手册](宏驱动开发.md#强类型参数)。

### 实例生命周期与多通道调用

对象身份与复用见[实例管理](instances.md)和[Config 身份](config-identity.md)。TB 使用普通操作 Input[0] 的“共享”标记解码多通道，每个槽位仍定位同一份 Config。其他平台直接构造结构化 MultiChannelInvocation，无须使用 TB 文本标记。跳过、广播、首错停止与结果顺序沿用原契约。

## 平台适配器

### 接口

平台实现 PlatformAdapter，并通过 node 的 platforms(...) 编译注册。提供稳定 platform 编号与 identifier，以及 decode/encode；需要支持枚举时实现 format_parameter/normalize_parameter。无自动枚举的旧适配器默认保留手写描述，枚举能力必须明确实现。

```rust,ignore
#[miresidian::node("示例节点", platforms(MyPlatformAdapter))]
struct ExampleNode;
```

节点宏自动包含 TB 适配器。Rust 宿主可显式构造 PlatformRegistry；直接 NodeRuntime::invoke_with 接收适配器对象，即使未在注册表内，候选渲染和输入转换也必须使用该对象。DLL 数值选择则仅接受已注册适配器。

### 平台选择

ABI v10 只通过调用请求的 platform 字段确定平台。0 是 MIRESIDIAN_PLATFORM_DEFAULT，通常对应 TB；自定义默认注册表可以指定自己的固定默认值。存在活跃对象时切换平台仍被拒绝，不会重绑定对象或隐藏地创建第二份运行时。

原生 Rust invoke_command 跳过平台 decode/normalize/encode，接收已规范化的结构化值；其元数据查询使用已绑定或默认注册适配器。原始 definition/signature 描述方法不猜测平台；发布给宿主时使用带适配器的 *_for 方法。

### 当前兼容平台

内置 TB 的共享标记、参数文本及响应拼接仅属于该平台。框架不因业务枚举新增具体通信依赖；NodeEnum 生成代码位于节点编译产物，框架不会内置业务枚举。

## 执行器与模块加载引用（0.15.1）

并发、截止时间、取消和卸载要求见[架构](architecture.md#并行与异步执行)。Linux 动态库模拟不能替代 Windows 原生宿主或真实硬件验收。

## Broadcast 示例（0.16.0）

广播执行源与跳过规则见[实例管理](instances.md#broadcast-的执行源与输出规则0160)。

## batch 与格式选择输入

batch/format 不改变本 ABI 的两个参数数组；框架选择器仍按元数据查询到的位置传入。见[format](../features/format/README.md)与[batch](../features/batch/README.md)。
