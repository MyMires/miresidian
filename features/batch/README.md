# miresidian_batch

`miresidian_batch` 是 Miresidian 参数级顺序批量调用的无状态支持 crate：它校验多个整数数组按位置对齐时的广播规则，并把每次标量调用的输出按槽位聚合。

节点项目不直接依赖此 crate；通过根 crate 启用：

```toml
miresidian = { version = "=0.17.0", features = ["batch"] }
```

## 节点接入

节点通过根 crate 启用 feature，并仅在普通业务 operation 的具体 Input 参数上标记 `batch`：

```rust
#[miresidian::operation("读取")]
#[miresidian::output("值")]
fn read(
    &mut self,
    #[miresidian::input("站号", batch)] station: i32,
    #[miresidian::input("通道", batch)] channel: i32,
    #[miresidian::input("倍率:1")] scale: i32,
) -> MiresidianResult<i32> {
    self.read_one(station, channel, scale)
}
```

业务方法始终接收单个基础整数；宏生成的包装层负责数组解析、调用和输出聚合。`batch` 不可用于 `init_device`、`close_device`、`get_port_no`、任何 Config 参数或实例主键。

## int[] 输入与顺序执行

标记参数使用 `int[]` 语义：`1` 是单项，`1|3|5` 是指定项，`1-5` 是递增范围，`1-3|5` 可混用。框架取所有 batch 参数的最大长度；等于最大长度的数组按位置组成记录，长度为一的数组自动广播。其他长度组合返回 `INVALID_ARGUMENT`。

例如 `站号=1`、`通道=1|3|5` 依次调用 `(1, 1)`、`(1, 3)`、`(1, 5)`；`站号=1|2`、`通道=1|3` 依次调用 `(1, 1)`、`(2, 3)`。未标记的 `倍率` 在每轮使用原输入与原默认值规则。

第一版固定顺序执行；首个失败立即停止，已经完成的副作用不回滚。每个输出槽位按调用顺序编码为可逆竖线集合。不提供并行、执行方式配置或笛卡尔积。

## 与 instances 的边界

`instances` 先用 Config/Handle 定位唯一节点对象；随后 `batch` 只在该对象上展开普通 Input。批量参数不能跨 Config/Handle 选择多个对象。平台多通道和 `|` 数组文本是不同维度：每个有效平台通道分别运行其内部顺序 batch。
