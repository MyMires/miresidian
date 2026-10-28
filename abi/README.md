# miresidian_abi

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_abi` 定义 Miresidian 节点 DLL 与宿主之间的稳定 C ABI：状态码、公共 FFI 类型、入口名称与 ABI 版本。

通常节点项目只需依赖 `miresidian`。只有实现宿主适配层、检查 ABI 兼容性，或需要直接使用公共 FFI 类型时，才直接引入本 crate。

```toml
[dependencies]
miresidian_abi = "0.11.1"
```

完整 C ABI 调用约定见[C ABI](../docs/c-abi.md)。
