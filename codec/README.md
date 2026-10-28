# miresidian_codec

`miresidian_codec` 提供 Miresidian 的跨语言值模型、参数解析，以及基础类型、数组、文本编码、HEX 与 Base64 编解码。

节点项目通常通过 `miresidian` 使用这些能力；需要独立处理平台输入输出时，也可以直接依赖本 crate。

```toml
[dependencies]
miresidian_codec = "0.11.0"
```

数据格式与编码规则见框架仓库的 `docs/data-codec.md`。
