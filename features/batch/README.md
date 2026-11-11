# miresidian_batch

通用业务类型的批量记录解析、长度对齐和输出聚合。本目录源码候选为 **0.3.2**，精确依赖 codec **0.18.0**，由框架 **0.33.0** 接入。

本次仅同步 instance 能力名称的组合指导，批量解析与聚合行为保持。上述版本尚未发布；独立节点采用已发布精确版本并启用根 crate 的 `batch` feature，不用本地覆盖。

## 按原方法类型解析

```rust,ignore
#[miresidian::operation("设置")]
fn set(
    &mut self,
    #[miresidian::input("名称", batch)] name: String,
    #[miresidian::input("开关", batch)] enabled: bool,
    #[miresidian::input("序号", batch)] index: u16,
) -> MiresidianResult<()> {
    self.workflow.set(name, enabled, index)
}
```

字符串和布尔直接支持 batch，无需 format。普通 batch 依赖参数类型的 NodeInput、NodeCollectionElement、Clone，类型别名和完整路径不影响判断；自定义类型须实现自己的集合语法，不自动构造任意嵌套集合协议。

| 类型 | 输入形式 |
| --- | --- |
| 整数 | 单值、竖线列表、非负递增范围、混合形式；`1-3\|5\|8-10` 展开为 1、2、3、5、8、9、10 |
| f32/f64 | 单值、列表、科学计数法和负值；拒绝范围，不推断步长 |
| String | 单个字符串或列表；`1-3` 保持字符串 |
| bool | 单值/列表，复用 true/false、1/0、是/否、真/假解析 |
| char | 单字符或字符列表 |

列表中的竖线和反斜杠分别转义为 `\|`、`\\`。空文本是一条空记录，由目标类型判断是否合法；String 可得到空字符串，数值解析为零，bool 空白报错。业务描述不补默认值，缺参报错；普通 Vec 的空集合规则不变。

## 对齐与执行

最长记录列决定调用次数；长度一广播，其余必须等长，按位置配对，不做笛卡尔积。例如 `名称=A|B|C`、`开关=true`、`序号=1-2|4` 调用 `(A,true,1)`、`(B,true,2)`、`(C,true,4)`。未标记 Input 复用实际输入值。

批量输入解析及长度校验先于业务调用；固定顺序执行，首个业务或输出转换错误停止，已有副作用不回滚。输出按槽位聚合为可逆竖线集合。batch 不用于 Config、初始化、关闭、探测或实例主键。instance 先定位一个对象，batch 随后在该对象上运行，不提供多对象分发或并行。

## 与 format 解耦组合

本 crate 不依赖 format，format 也不依赖本 crate；共同复用 codec，宏负责组合。`decode_batch<T>` 按目标类型解析普通输入；`batch_records` 只拆转义记录；`batch_length` 和 `batch_outputs` 分别对齐、聚合。

需要编码转换时额外启用 format。数据与格式选择可分别传单值或列表，长度一广播、其他等长；固定单编码不生成选择器。每条编码记录解码为一个原方法参数，不再次拆分解码结果或展开其中的整数范围。输出格式也可逐条选择，然后按槽位聚合。完整用法见 [format](../format/README.md)。

## 验证

覆盖独立 batch、独立 format、组合 feature；整数混合、文本/布尔、别名、空记录、转义、格式单值/列表、固定格式无选择器、多输入槽位、输出聚合、输入失败无业务执行及同步/async。
