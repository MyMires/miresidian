# miresidian_codec

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_codec` 提供 Miresidian 的跨语言值模型、参数解析，以及基础类型、数组、文本编码、HEX 与 Base64 编解码。

节点项目通常通过 `miresidian` 使用这些能力；需要独立处理平台输入输出时，也可以直接依赖本 crate。

本检出源码为 **0.18.0**，尚未发布；独立项目须选择已发布的精确版本，不将本候选作为可直接安装版本。

数据格式与编码规则见[数据编解码](../docs/宏驱动开发.md#数据编解码)。

0.15.0 起，数值显式空白为零、完整候选文本不取首项；业务参数缺参报错。GBK 解码禁用 BOM 探测与非法字符替换。低层 decode_value 等函数的显式 default 参数仍可用于框架策略，不从描述自动推导。

0.17.0 使用 NodeInput::normalize_text 为配置身份和租约提供纯规范文本；内置数值、bool、格式、Option/Vec 使用各自类型语义，String/自定义类型默认保留文本。选项映射识别转义后的完整集合元素。

## 通用节点类型（0.17.0 源码）

公开接口为 `NodeInput`、`NodeOutput`、`NodeCollectionElement`、`NodeScalar` 和 `NodeValue`，根框架同时重导出；不保留旧设备限定名称。自定义输入、输出与集合实现分别使用 `parse_text` / `normalize_text`、`into_output`、`parse_collection` / `format_collection`。

以下是本源码接口示例，须待目标版本发布后再用于独立项目：

```rust
use miresidian_codec::{MiresidianError, MiresidianResult, NodeInput};

enum Priority { Normal, Urgent }

impl NodeInput for Priority {
    fn parse_text(value: &str, field: &str) -> MiresidianResult<Self> {
        match value {
            "Normal" => Ok(Self::Normal),
            "Urgent" => Ok(Self::Urgent),
            _ => Err(MiresidianError::InvalidArgument {
                field: field.to_owned(),
                message: "优先级不受支持".to_owned(),
            }),
        }
    }
}
```

节点适配接口由宏实现为 `miresidian::Node`，不属于本 codec crate。完整更名表及升级边界见[框架 0.29.0 迁移](../docs/releases/0.29.0.md)。0.18.0 新增 OPTIONS 结构化候选；枚举派生由根框架的 NodeEnum 提供，具体用法见[节点手册](../docs/宏驱动开发.md#枚举自动转换与候选)。

## 0.18.0 结构化枚举候选

EnumOption 保存 value/display，不携带平台分隔符。NodeInput::OPTIONS 和 NodeCollectionElement::OPTIONS 默认空切片，旧实现无需补写；Option 和 Vec 转发元素候选。自定义节点通常用根框架的 NodeEnum 派生自动生成，不在 codec 内声明业务枚举。

normalize_enum_text 将具体显示值映射为实际值，并遵循标准竖线集合转义；它不解析平台候选描述、不选首项。平台适配器先处理本平台输入表示，再把实际值交给派生 parse_text；已有手写描述由对应平台原样展示。完整用法和默认/错误规则见[节点手册](../docs/宏驱动开发.md#枚举自动转换与候选)。
