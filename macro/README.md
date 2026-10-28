# miresidian_macro

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_macro` 是 Miresidian 的过程宏实现 crate，负责生成节点契约、调用分发、生命周期调度和 C ABI 入口。

节点项目应依赖并使用 `miresidian` 重导出的宏，而不需要直接依赖本 crate：

```rust
use miresidian::{config, input, node, operation, output};
```

宏标注和节点实现约束见[宏驱动开发](../docs/宏驱动开发.md)。

`node` 保留在结构体上，生成节点身份、注册绑定和 C ABI；每个 `operation` 独立生成方法契约，支持同模块多个固有 impl。初始化、关闭和可选探测同样使用 operation，节点不再需要 Default。
