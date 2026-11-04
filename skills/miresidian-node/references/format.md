# format：显式格式编解码

选择 `format` 时读取。适配版本见 [主文档](../SKILL.md#适配版本)。基础 codec 解析默认可用，节点无需直接依赖内部 crate。

```toml
[dependencies]
miresidian = { version = "=0.20.1", features = ["format"] }
```

format 决定编解码，batch 决定重复调用，两者独立启用。

## 标注与槽位

委托业务对象的契约片段：

```rust
#[miresidian::operation("收发")]
#[miresidian::output("接收数据", format(
    formats = [hex, utf8, base64], selector = "接收数据格式"
))]
fn exchange(
    &mut self,
    #[miresidian::input("发送数据", format(
        formats = [utf8, hex, base64], selector = "发送数据格式"
    ))]
    data: Vec<u8>,
) -> MiresidianResult<Vec<u8>> {
    self.device.exchange(data)
}
```

Input 顺序为发送数据、发送数据格式、接收数据格式；Output 只有接收数据。业务方法只接收内部数据，此例每次处理一份数据。

- formats 必填、非空、无重复，允许 ascii/gbk/utf8/utf16le/hex/base64；第一项为默认，选择输入缺失/空白用默认，非法值报错。
- 单格式如 `format(formats = [hex])` 固定格式，不生成选择输入，不接受 selector；多格式须声明 selector。
- 输入选择器紧随对应数据，输出选择器按输出声明顺序追加到全部方法输入后。平台选项名为 ASCII/GBK/UTF8/Unicode/HEX/Base64，解析沿用框架格式别名。
- 只支持普通业务 Input/Output，不支持 Config、生命周期或探测。逐项输出标注依次对应元组元素，不能与单标注多名称混用。

## 类型与字节契约

内置实现支持上述六种格式，按内部类型解释数据：

| 内部类型 | 表示与转换 |
| --- | --- |
| Vec<u8> | 原始字节；字符格式转换编码字节，HEX/Base64 编解码原始字节 |
| String、char | 规范 UTF-8 文本；字符格式严格校验可表达性，HEX/Base64 表达 UTF-8 字节 |
| 整数、f32/f64、bool | 规范 UTF-8 标量文本；复用默认解析，bool 输出小写 true/false |
| 数值/布尔 Vec<T>（u8 除外） | 单条记录用空白分隔元素，不用竖线、范围或 JSON；空文本为空数组 |
| Option<T> | 空字节为 None，其余按 T 转换；None 与 Some(空值) 不可区分 |
| 自定义类型/枚举 | 实现 `miresidian::format::{FormatInput, FormatOutput}`，声明字节契约；RAW_BYTES 默认 false，原始字节类型设 true；batch 输入另需 Clone |

例如 i32 的 HEX `31 32` 表示文本 12，Vec<u8> 得到两个字节；String 的 GBK 输入保留文本，Vec<u8> 得到 GBK 字节。无内置 Vec<String>、嵌套数组或 char 数组实现；缺少 trait 时编译失败。普通标量无需额外 format 标注。

## 与 batch 组合

需要重复处理时，在同一依赖启用 `["format", "batch"]`，并标记数据参数：

```rust
#[miresidian::input("发送数据", batch, format(formats = [hex]))]
data: Vec<u8>,
```

- 先按转义竖线拆记录，再逐条解码；例如 `31 32|44 45` 先后传入两份字节。业务仍处理单条记录，输出先逐次编码，再按槽位聚合，不展平记录内数组。
- 外层 batch 将内容中的 `|`、反斜杠转义为 `\|`、`\\`；非 batch 不加此层。Base64 输出沿用 `base64:` 前缀。
- 空输入是一条空记录，`|41` 是两条；结合请求记录数解释空结果。单条数值数组如 `1 2`，批量如 `1 2|3 4`。
- 按 [batch 规则](batch.md#展开与执行) 对齐、广播、顺序执行并首错停止；普通数值 batch 不需要 format。

验收所选格式的正常转换、默认/非法选择、选择器槽位和相关编码错误；组合 batch 时核对边界、转义、空记录及逐槽输出。输入转换失败不执行业务；输出转换失败可能在副作用后发生，停止后续记录，不回滚或重试。字符编码严格报错，不替换非法字符，同步/async 规则一致。
