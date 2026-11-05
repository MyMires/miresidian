# format：显式格式编解码

选择 `format` 时读取。适配版本见 [主文档](../SKILL.md#适配版本)。基础 codec 解析默认可用，节点无需直接依赖内部 crate。

```toml
[dependencies]
miresidian = { version = "=0.22.0", features = ["format"] }
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

同时启用 `["format", "batch"]` 后，对需要展开的数据参数标注 batch。下面方法的原参数是 String：

```rust
#[miresidian::input("文本", batch, format(
    formats = [utf8, hex, base64], selector = "文本输入类型"
))]
text: String,
```

普通入口对应 `文本` 和紧随其后的 `文本输入类型` 两个 Input：

| 数据 | 格式 | 每次业务参数 |
| --- | --- | --- |
| `A\|42\|Qw==` | `UTF8\|HEX\|Base64` | A、B、C |
| `41` | `UTF8\|HEX` | 41、A，数据单值广播 |
| `41\|42` | `HEX` | A、B，格式单值广播 |

表中竖线是记录分隔符。先按转义竖线拆数据/格式列并对齐，再逐条解码；空格式项沿用允许列表第一项。未知或不允许的格式、列长度不匹配、无效业务值均在业务调用前拒绝。与其他 batch 参数按最长列对齐；未标记 batch 的输入格式选择器仍是一个格式，不能传列表。

固定编码不生成选择输入，也不接受 selector：

```rust
#[miresidian::input("文本", batch, format(formats = [hex]))]
text: String,
```

只传一个 Input `41|42`，依次得到 A、B；没有额外“文本输入类型”。只使用 format、不使用 batch 时，同样支持固定编码而不生成选择器。

- 原方法类型决定单条格式解码：String/布尔/数值得到对应值，Vec<u8> 得到一份完整字节记录；例如 `31 32|44 45` 是两份字节。解码结果内的竖线不再拆分，不展平记录内集合。
- 普通整数 batch 仍支持 `1-3|5|8-10`；format 中每个编码记录表示一个业务值，不把解码出的 `1-3` 再解释为批量范围。可在同一方法中组合整数范围列与其他编码数据列。
- 内容中的竖线、反斜杠在外层分别转义为 `\|`、`\\`；空数据是一条空记录，`|41` 是两条。结合记录数解释空输出。
- batch 方法的输出格式选择器也支持单值广播或等长列表，位置仍在全部方法 Input 后；输出逐条编码后按槽位聚合。Base64 输出沿用 `base64:` 前缀。
- 调度与编码职责解耦：batch 和 format crate 互不依赖，宏组合各自的记录/单条 API。兼容函数 decode_formatted_batch 只按固定一种格式解码记录，不负责对齐或重复调用。

按 [batch 规则](batch.md#展开与执行) 验收。另覆盖无 batch 的单次转换、固定编码无选择器、输入/输出选择器槽位、非法格式、转义/空记录、输出失败后的停止及 async。输入解析失败不执行业务；输出失败可能在业务副作用之后，不回滚或盲重试。
