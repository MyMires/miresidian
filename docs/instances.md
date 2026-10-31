# 实例管理与多通道执行

[文档中心](README.md) · [宏驱动开发](宏驱动开发.md) · [平台调用契约](c-abi.md)

## 默认单实例

未声明 `instances(...)` 的节点最多保留一个对象；只开启 Cargo feature 不改变此默认行为。同一份规范化业务 Config 初始化会复用当前对象；不同 Config 初始化先预解析参数，成功后关闭并析构旧对象、释放租约，再构造新对象。预解析失败保留旧对象；关闭失败阻止新构造；旧对象释放后构造失败保持未初始化。普通调用和关闭使用当前对象的完整业务 Config 校验目标，不会隐式切换设备。

无配置的单实例节点可用空 Config 初始化。默认节点不接受多通道请求，也不发布执行方式配置。

## 独立 crate 与 feature

通用能力位于独立 `miresidian_instances` crate，依赖通用编解码 crate，不依赖根运行时、节点宏或通信实现。基础策略与单实例容器始终可用；根 feature `instances = ["miresidian_instances/runtime"]` 启用多实例容器及多通道调度。主框架只负责解析节点 Config、构造/关闭对象并将对象调用接入通用调度。

## feature 与节点声明

```toml
[dependencies]
miresidian = { version = "=0.16.1", features = ["instances"] }
```

```rust,ignore
// 单实例，允许多通道输入。
#[miresidian::node("设备", instances(
    mode = single,
    execution = selectable,
    default_execution = sequential
))]
struct Device;

// 多实例，以 Config[0] 为主键。
#[miresidian::node("设备", instances(
    mode = multiple,
    binding = config,
    execution = selectable,
    default_execution = sequential
))]
struct Device;

// 多实例，以框架 Handle 为主键，固定顺序执行。
#[miresidian::node("设备", instances(
    mode = multiple,
    binding = handle,
    execution = sequential
))]
struct Device;
```

上面是三种独立声明示意；一个节点模块仍只声明一个节点，初始化、关闭和业务方法继续由 `operation` 标注。

| 配置 | 规则 |
| --- | --- |
| `mode` | 必填，`single` 或 `multiple` |
| `binding` | `multiple` 必填，`config` 或 `handle`；`single` 不接受 |
| `execution` | 必填，`sequential`、`parallel`、`broadcast` 或 `selectable` |
| `default_execution` | `selectable` 必填，为三种具体方式之一；固定方式不接受 |

声明 `instances(...)` 却未开启依赖的 feature 会编译失败。重复、未知、缺失或无效配置也会编译失败；节点 crate 自身同名 feature 不能代替框架依赖的 feature。

## Config[0] 主键（新节点项目强制规则）

初始化的第一个业务 Config 参数必须是对象主键，位于 `Config[0]`，例如串口端口名或设备地址。其他配置如波特率、超时不参与对象身份。宏拒绝没有业务 Config 的 `multiple + config` 节点；业务上哪个字段是主键由节点作者正确放在第一位，框架不根据名称猜测。

判空先于默认值补齐：未提供 Config[0]、空字符串、纯空白或只有参数名前缀（如 `地址:`）时，InitDevice 成功返回空输出，跳过创建、参数预解析和租约获取，保持已有对象不变。后面的业务参数或框架字段非空，也不能代替主键；元数据中的默认值不能把空通道变成对象。

非空主键按现有名称前缀及选项显示值规则规范化。相同主键且完整业务配置一致时复用对象；相同主键却携带不同配置时返回 `INVALID_ARGUMENT`，不创建第二个对象。不同主键分别登记对象。业务调用和关闭只用主键定位，方法中的 config 参数读取对象保存的规范化业务配置。

## Handle 关联

Handle 由平台放在 Config 中，框架自动在业务 Config 之后追加 `Handle` 元数据项，不进入节点初始化参数和业务 Config 索引。

```text
业务 Config: [设备, 波特率]
实际 Config: [设备, 波特率, Handle, ...其他框架字段]

["COM3", "9600", "H1"] → 对象 H1
["COM3", "9600", "H2"] → 独立对象 H2
```

相同 Handle 和一致配置复用对象；相同 Handle 携带不同配置或不同执行方式初始化时返回冲突。不同 Handle 即使业务配置一样也不合并，不提供多个标识指向同一对象的别名规则。

Handle 缺失、空白或只有 `Handle:` 前缀时，初始化成功返回空输出，不创建或替换对象。业务调用和关闭时 Handle 为空返回 `INVALID_ARGUMENT`；不存在或已关闭时返回 `UNKNOWN_CHANNEL`，不回退到其他对象。Handle 按文本区分大小写。

## 框架 Config 布局

描述发布与解析共用 `ConfigLayout`，顺序固定：

1. 业务 Config，主键位于下标 0；
2. Handle（仅 handle 关联）；
3. 资源访问方式（仅 selectable 租约）；
4. 平台适配（普通入口发布，显式 invoke_with 入口不发布、不接受）；
5. 执行方式（仅 execution = selectable）。

普通入口保留省略平台 id 的兼容路径，随后执行方式可直接占其位置。Handle 位于业务参数之后，必须保留前方业务参数位置，必要时填空以采用非主键参数默认值。所有框架字段均在调用节点前剥离，不参与业务主键。获取固定操作元数据不需要传入 Handle。

## 多通道执行

框架称为多通道调用；TB 的 `共享` 标记和 `共享执行方式` 显示文本只作平台兼容约定。

| 策略 | 行为 |
| --- | --- |
| sequential | 按平台输入顺序逐项调用同一对象 |
| parallel | 各有效输入调度为 Future，同一对象的完整方法调用由状态锁互斥保护，开始顺序不保证 |
| broadcast | 固定执行通道1一次，将完整结果复制到有效通道；其他输入不必相同，空通道跳过 |
| selectable | 初始化时选择并保存在对应对象上；不同对象可有不同方式 |

固定方式不发布、不接受执行方式 Config 项。selectable 缺省或空白使用宏声明的默认值；TB 兼容的未知文本仍回退 Sequential。相同主键重复初始化时必须使用一致的解析结果。

一次多通道请求只携带一份 Config，按 Config[0] 或 Handle 定位一次对象，全部有效输入使用它。输入位置不关联对象创建顺序，输入数量不受对象数量限制。空白和非法参数数目的跳过规则、输出对齐和执行错误契约沿用 TB 兼容协议；并行失败不表示其他输入没有执行。

Rust 使用 `RuntimeInvocation::MultiChannel(MultiChannelInvocation { operation, config, slots })` 和 `RuntimeResponse::MultiChannel`。旧类型名 `SharedInvocation`、`SharedResponse` 保留别名；旧 Shared 枚举变体应迁移为 MultiChannel。

## 0.14.0 迁移

- 需要保留旧版多个 Config 对象同时存在的节点，开启 feature 并显式声明 `multiple + config`，将真实业务主键放在 Config[0]。
- 需要 TB 多通道输入的单实例节点，声明 `single` 和执行策略。
- 不再无条件追加执行方式；调用方查询 GetInitConfig，不固定假设参数数量。
- 串口节点 0.10.0 显式选择 multiple/config/selectable，端口名称为主键；同端口不同设置初始化报冲突，空端口跳过。
- C 导出签名、JSON 格式和返回文本内存契约不变，C ABI 保持 v7；租约协议保持 v2。

## 0.15.0 并发契约

同一 DLL 的运行时以共享内部状态管理实例，普通调用不再持有覆盖业务执行的全局锁。
不同 Config[0] 或 Handle 对应的实例可同时执行；同一实例的请求互斥。
多通道请求整体取得实例执行权，批次内部执行期间其他请求不能插入，执行权持续到全部已开始的输入结束。
Parallel 仍是同实例输入调度，输出保持输入位置；不提供同实例业务方法重叠。0.16.0 起广播固定调用通道1一次，随后复制输出，不重复业务副作用。

显式跨实例批次使用 `NodeRuntime::invoke_batch` / `invoke_batch_async`，每项 `Invocation` 自带目标 Config。
结果与请求一一对应，各项错误单独返回，全部请求结束后返回；不存在按对象创建顺序选择目标的行为。
请求数受 `ExecutionLimits::max_batch` 限制，超过容量在开始执行前拒绝。

关闭先阻止实例的新业务执行，等待完整在途请求结束，然后关闭、析构并释放租约。
正在关闭的对象保持主键占位；同键重建在旧对象清理结束后执行。
初始化和关闭事务当前按运行时串行协调，独立实例的业务执行不被该生命周期锁阻塞。
全局关闭拒绝新业务与初始化，等待正在构造的对象完成并清理，再返回；完成后运行时可重新初始化。
不同实例指向同一物理资源时，实例锁不提供资源级事务协调；租约协议含义保持不变。

## Broadcast 的执行源与输出规则（0.16.0）

Broadcast 的含义是“一次业务执行，多通道复用输出”。执行源固定为通道1，即共享输入槽位下标0；TB 的 Input[0] 为“共享”标记，通道1对应其后的 Input[1]，不是标记本身。

- 只使用通道1的参数执行业务一次；其他有效通道可以传不同值，不再校验输入一致。
- 将通道1编码完成的整份 output 复制到每个有效通道，包括多字段、数组和空返回值；输出槽位数量与输入一致。
- 空白/被标记跳过/参数数量不合法的目标通道继续跳过，TB 输出对应空字符串。
- 通道1被标记跳过或参数数量不合法时，整个广播都跳过，不自动选择后续有效通道。
- 通道1的类型解析或业务执行失败时，整个调用报告通道1错误；不重试其他输入，也不回滚已有副作用。
- 结构化请求的 Some([]) 是否有效按方法参数及默认值判断，不等同于 None；零参数方法允许 Some([]) 作为通道1。
- 结果复制期间仍持有该实例的整个请求执行权，其他请求不能插入批次。

以下 R 是执行一次 Work(10) 得到的完整输出，数字仅表示一个业务参数：

| 输入槽位 | 实际业务调用 | 输出槽位 |
| --- | --- | --- |
| [10, 20, 30] | Work(10)，1次 | [R, R, R] |
| [10, 20, 空, 30] | Work(10)，1次 | [R, R, 空, R] |
| [空, 20, 30] | 0次，不切换源 | [空, 空, 空] |
| 通道1参数数量合法但类型错误 | 通道1解析失败，业务0次 | 整体返回错误 |
| 通道1业务失败 | 业务1次 | 整体返回错误，不改用通道2 |

有效目标的判断复用既有槽位规则（空白、参数个数与默认值），不会对其他通道执行类型解析或业务；其值不会影响通道1结果。
同一规则覆盖通用同步调度、运行时 Future 调度、TaskService 提交的多通道请求、结构化多通道调用和两个现有 C ABI 入口。普通单通道请求仍一次请求执行一次；显式跨实例批次的各项仍独立执行，不会合并成一次广播。

### 从旧广播语义迁移

0.15.x 及之前的实现把有效输入逐项调用，文档和测试也曾据此描述；0.16.0 根据明确的执行次数要求将其校正为通道1单次执行和结果复制。
若业务确实需要重复执行相同参数，应使用 Sequential；Broadcast 不再产生每目标一次的重复副作用。
C 导出签名、JSON 编码、返回文本生命周期和 Config 布局不变，C ABI 保持 v7。
