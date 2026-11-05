# miresidian_codec

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_codec` 提供 Miresidian 的跨语言值模型、参数解析，以及基础类型、数组、文本编码、HEX 与 Base64 编解码。

节点项目通常通过 `miresidian` 使用这些能力；需要独立处理平台输入输出时，也可以直接依赖本 crate。

```toml
[dependencies]
miresidian_codec = "=0.16.0"
```

数据格式与编码规则见[数据编解码](../docs/宏驱动开发.md#数据编解码)。

0.15.0 起，数值显式空白为零、完整候选文本不取首项；业务参数缺参报错。GBK 解码禁用 BOM 探测与非法字符替换。低层 decode_value 等函数的显式 default 参数仍可用于框架策略，不从描述自动推导。

0.16.0 的 DeviceInput::normalize_device_text 为配置身份和租约提供纯规范文本；内置数值、bool、格式、Option/Vec 使用各自类型语义，String/自定义类型默认保留文本。选项映射识别转义后的完整集合元素。
