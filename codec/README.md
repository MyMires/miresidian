# miresidian_codec

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_codec` 提供 Miresidian 的跨语言值模型、参数解析，以及基础类型、数组、文本编码、HEX 与 Base64 编解码。

节点项目通常通过 `miresidian` 使用这些能力；需要独立处理平台输入输出时，也可以直接依赖本 crate。

```toml
[dependencies]
miresidian_codec = "0.11.2"
```

数据格式与编码规则见[数据编解码](../docs/宏驱动开发.md#数据编解码)。
