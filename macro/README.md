# miresidian_macro

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_macro` 是 Miresidian 的过程宏实现 crate，负责生成节点契约、调用分发、生命周期调度和 C ABI 入口。

节点项目应依赖并使用 `miresidian` 重导出的宏，而不需要直接依赖本 crate：

```rust
use miresidian::{config, input, node, operation, output};
```

宏标注和节点实现约束见[宏驱动开发](../docs/宏驱动开发.md)。

`node` 保留在结构体上，生成节点身份、注册绑定和 C ABI；每个 `operation` 独立生成方法契约，支持同模块多个固有 impl。初始化、关闭和可选探测同样使用 operation，节点不再需要 Default。

0.15.0 支持 operation 标注 async 方法，生成原生业务 Future 桥；构造、关闭和探测的 async 桥由生命周期线程等待。同步 DLL 入口共享内部并发运行时。参见[异步方法](../docs/宏驱动开发.md#异步方法)。


## 类型驱动的 batch 与 format 组合

普通业务 Input 的 batch 不再按 Rust 类型名称限制为数值；生成代码使用原参数类型的 DeviceInput、DeviceCollectionElement 与 Clone。整数保留单值/列表/范围/混合，字符串、布尔、字符和别名使用其类型解析。format 的单条编解码独立于 batch；组合宏对齐数据列和格式列，支持格式广播/逐条选择及输出格式列。固定单编码不生成选择输入，原有参数顺序与 metadata 布局保持。

宏只组合各功能 API；batch 与 format 两个实现 crate 不互相依赖。输入解析/选择/长度校验先于业务，保持顺序、首错停止、逐槽输出聚合和同步/async 桥接。

0.20.0 起，业务 Config/Input 描述仅原样发布，不生成运行时默认值；缺参报错，数值显式空白为零，具体选项显示值仍映射。format 选择器等框架生成的显式默认值继续生效。

0.21.0 按初始化 Config 的原 Rust 类型生成归一化回调，根运行时和 lease 参数共用；类型别名通过 trait 分派，不按类型名称猜测。
