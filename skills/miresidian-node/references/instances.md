# instances：实例与多通道

选择 `instances` 时读取。适配 [主文档基线](../SKILL.md#适配版本)，策略语义按节点精确版本核对；共用配置见 [能力依赖](node-development.md#能力依赖) 和 [Config 布局](node-development.md#config-布局)。

## 依赖与声明

```toml
[dependencies]
miresidian = { version = "=0.19.4", features = ["instances"] }
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
| selectable | 初始化选 Sequential/Parallel/Broadcast，default_execution 决定缺省/空值策略 |

多对象与多通道是独立维度：一次多通道请求仍按一份 Config 定位同一对象，parallel 不提供同一连接上的并行通信事务。

## 版本差异

- 框架 0.14.x–0.15.x 的 Broadcast 要求各有效通道输入一致，仍逐通道执行业务。
- 0.16.0 起改为通道1单次执行并复制结果；本页 0.19.4 示例使用此行为，旧版不能据此判断副作用次数。

## 验收

验收所选策略：config 关联验证双对象隔离、同键复用/冲突、空主键跳过和目标关闭；handle 关联验证元数据、对象定位、缺失/未知值及关闭隔离。多通道只验证声明的固定策略或需支持的 selectable 选项，核对输出槽位和副作用。
