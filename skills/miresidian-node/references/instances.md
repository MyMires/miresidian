# instances：实例与多通道

选择 `instances` 时读取。适配版本见 [主文档](../SKILL.md#适配版本)；共用配置见 [能力依赖](node-development.md#能力依赖) 和 [Config 布局](node-development.md#config-布局)。

## 依赖与声明

```toml
[dependencies]
miresidian = { version = "=0.21.0", features = ["instances"] }
```

仅启用 feature 仍是默认单实例；多实例/多通道还须节点策略标注。例如：

```rust
#[miresidian::node("示例节点", instances(
    mode = multiple,
    binding = config,
    execution = selectable,
    default_execution = sequential
))]
struct ExampleNode { /* 持有业务对象 */ }
```

- mode 必填。`single` 提供单对象多通道，不接受 binding；`multiple` 必须指定 `binding = config` 或 `handle`。
- config 以 Config[0] 主键关联：空主键跳过初始化，同键同配置复用，同键不同配置报错。handle 在业务 Config 后追加 Handle，业务签名不接收它；空 Handle 初始化跳过，调用/关闭不能以空值选择其他对象。
- execution 必填，规则如下；只有 selectable 声明 default_execution，并发布执行方式 Config。策略保存在初始化对象上，变更遵循复用、冲突或关闭重建规则。

| execution | 行为与约束 |
| --- | --- |
| sequential | 通道依次执行，不追加执行方式 Config |
| parallel | 分发通道任务，同一对象业务仍互斥，不保证开始顺序；不追加 Config |
| broadcast | 固定执行通道1一次，将完整结果复制给有效通道；通道1无效则整体跳过，不切换执行源 |
| selectable | 初始化选 Sequential/Parallel/Broadcast，default_execution 决定缺省/空值策略；未知文本回退 Sequential，调用方应传合法值 |

多对象与多通道是独立维度：一次多通道请求仍按一份 Config 定位同一对象，parallel 不提供同一连接上的并行通信事务。

## 对象复用与资源共享

| 初始化条件 | 对象结果 |
| --- | --- |
| config 关联，同 Config[0] 且规范化业务配置相同 | 复用一个对象，不重复构造或申请运行时租约 |
| config 关联，同 Config[0] 但其余业务配置不同 | 报冲突，不新建对象 |
| handle 关联，同 Handle、同业务配置 | 复用同一对象 |
| handle 关联，不同 Handle、业务配置相同 | 独立对象和业务状态 |
| 不同 DLL 使用相同配置 | 各 DLL 独立管理业务对象 |

重复 InitDevice 不增加对象引用次数，一次 CloseDevice 就关闭所定位的对象。已有对象的资源访问方式或执行方式不同，重复初始化报错，须先关闭再初始化。

独立对象可以持有同一资源的不同代理；物理连接的引用与事务由 [SharedResource 通信层](lease.md#共享连接接入) 管理。配置租约不提供这层复用，也不能用不同 Handle、项目名或 DLL 文件名绕过物理资源冲突。

## TB 多通道调用

以下假设唯一业务 Config 为连接地址，方法“读取寄存器”接收站号、通道两个必填 Input，返回一个值。沿用上例 multiple/config/selectable 策略；请求以 Rust Invocation 的 JSON 模型表示，C ABI 则分别传 function、config_json、input_json：

```json
[
  {
    "operation": "InitDevice",
    "config": ["COM3", "tb_lv_test_platform", "Broadcast"],
    "input": []
  },
  {
    "operation": "读取寄存器",
    "config": ["COM3", "tb_lv_test_platform", "Broadcast"],
    "input": ["共享", "1,2", "", "3,4", "1,2,3"]
  }
]
```

前三项 Config 分别为地址、平台、执行方式；组合其他能力时按 [Config 布局](node-development.md#config-布局) 调整。

- Input[0] 为“共享”，之后每项代表一个平台通道，按英文逗号拆为方法参数，参数内数组仍用竖线。TB 不提供逗号转义；需传含逗号文本或保留多输出边界时，选择普通单通道调用或结构化适配器。
- 空通道、任一参数空白、参数过多或缺少无默认值的尾项时跳过，输出保留对应空字符串；尾项均有默认值时允许省略。参数数目合法后的类型解析/业务错误使整个调用失败，并标注通道序号；已有副作用不回滚，Parallel 其他任务可能已执行。
- 本例只调用通道1的“读取寄存器(1,2)”一次；若结果为 R，平台 output 为 `["R", "", "R", ""]`。Broadcast 只检查其他目标通道的形状，不解析其业务值；通道1无效则全部跳过。
- 输出按输入通道保持数量和顺序，TB 将每个有效通道的多个输出以逗号连接为一个字符串。结构化多通道使用 MultiChannelInvocation 的 slots：None 表示跳过，Some([]) 按参数/默认值判断，零参数方法可用。

## 验收

验收所选策略：config 关联验证双对象隔离、同键复用/冲突、空主键跳过和目标关闭；handle 关联验证元数据、对象定位、缺失/未知值及关闭隔离。多通道验证所选策略的调用次数、输入编码、跳过/执行失败、输出对齐和副作用；Broadcast 覆盖通道1无效及其他输入不同值。
