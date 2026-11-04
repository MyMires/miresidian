# batch：顺序批量调用

选择 `batch` 时读取。适配版本见 [主文档](../SKILL.md#适配版本)；共用配置见 [能力依赖](node-development.md#能力依赖)。

## 依赖

用于同一已初始化对象按多条记录重复业务。依赖声明：

```toml
[dependencies]
miresidian = { version = "=0.20.1", features = ["batch"] }
```

## 参数标注

在普通 operation 的 Input 上标记，业务方法保持单条记录的参数类型；以下为委托业务对象的契约片段：

```rust
#[miresidian::operation("读取寄存器")]
#[miresidian::output("值")]
fn read_register(
    &mut self,
    #[miresidian::input("站号", batch)] station: i32,
    #[miresidian::input("通道", batch)] channel: i32,
    #[miresidian::input("倍率:1")] scale: i32,
) -> MiresidianResult<i32> {
    self.device.read_register(station, channel, scale)
}
```

## 展开与执行

- 整数支持单项 `1`、列表 `1|3|5`、递增范围 `1-5` 和混合 `1-3|5`；f32/f64 支持列表、负值和科学计数法，拒绝范围、不推断步长。
- 最长展开数组决定次数；长度一广播，其余必须等长，按位置配对，不做笛卡尔积。未标记 Input 按原值/默认值复用。
- 只标记普通业务 Input，不用于 Config、初始化、关闭或探测。顺序执行，首错停止，已有副作用不回滚；各输出槽位按调用顺序聚合为可逆竖线集合。
- 组合 [instances](instances.md) 时先定位一个对象，再执行各记录；batch 不展开 Config 主键/Handle，不等于多通道或并行。与 format 组合见 [记录与转义](format.md#与-batch-组合)。

## 验收

验收实际支持的输入语法、单值广播、等长对齐/不对齐、未标记参数复用、顺序、首错停止及逐槽聚合。组合 instances 时确认对象定位不变；同时有平台多通道时核对每个通道内的 batch 结果。
