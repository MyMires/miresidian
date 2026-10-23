# miresidian_macro

`miresidian_macro` 是 Miresidian 的过程宏实现 crate，负责生成节点契约、调用分发、生命周期调度和 C ABI 入口。

节点项目应依赖并使用 `miresidian` 重导出的宏，而不需要直接依赖本 crate：

```rust
use miresidian::{config, input, node, node_impl, operation, output};
```

宏标注和节点实现约束见框架仓库的 `docs/宏驱动开发.md`。
