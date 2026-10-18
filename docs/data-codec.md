# 数据编解码规范

Miresidian 的 `codec` 是根框架能力，负责强类型参数、平台字符串数组与原始字节之间的稳定转换。
设备节点只能声明 Rust 类型或调用 `decode_text_to_bytes`、`encode_bytes_to_text`、`transcode_text`；不得各自实现
HEX、Base64、字符编码或竖线数组语法。

本规范只覆盖平台文本与原始字节之间的转换。串口租约协议中的数据位、校验位等内部字段编码不属于本模块，也不会使用本模块的公开 API。

属性宏 crate 不实现这些规则。它只展开为对 `miresidian::codec::decode_value` 与
`miresidian::codec::encode_value` 的调用；前者统一处理 `Config[]` 与 `Input[]`，因此每个
使用 `#[node_impl]` 的节点在运行时自动复用本规范。

## 数据格式

| 名称 | Rust 枚举 | 说明 |
| --- | --- | --- |
| `ASCII` | `ByteTextFormat::Ascii` | 严格 7 位 ASCII。它是 GBK 的子集。 |
| `GBK` | `ByteTextFormat::Gbk` | 明确的 GBK 中文编码。 |
| `UTF-8` | `ByteTextFormat::Utf8` | 标准 UTF-8。也接受 `UTF8` 别名。 |
| `Unicode` | `ByteTextFormat::Utf16Le` | 固定表示 UTF-16LE；也接受 `UTF-16LE`。 |
| `HEX` | `ByteTextFormat::Hex` | 原始字节的十六进制文本。 |
| `Base64` | `ByteTextFormat::Base64` | 原始字节的标准 Base64 文本。 |

不支持 `ANSI`。它在 C# 中通常指系统默认代码页，会随 Windows 区域设置变化，不能作为可重复的
跨语言测试契约。需要英文单字节文本时使用 `ASCII`；需要中文兼容编码时使用 `GBK`。

转换始终遵循“源格式文本 → 原始字节 → 目标格式文本”。例如：

```rust
use miresidian::{ByteTextFormat, transcode_text};

let hex = transcode_text("测试", ByteTextFormat::Gbk, ByteTextFormat::Hex)?;
assert_eq!(hex, "B2 E2 CA D4");
```

任何无法无损表示的转换都会返回 `DriverError::InvalidArgument`，不会以替换字符或 `?` 静默继续。
因此 GBK 可以读取和表达 ASCII，GBK 转 ASCII 遇到中文必定失败。

## 强类型参数

属性宏生成的参数解析通过 `DeviceInput` 和 `DeviceOutput` 完成。已支持：

- `String`、`bool`、`char`；
- 所有常用有符号、无符号整数和 `f32`、`f64`；
- `Option<T>`、一维 `Vec<T>` 与元组输出；
- `ByteTextFormat`；
- `Vec<u8>` 的 Base64 规范表示。

整数除十进制外还支持 `0x` 十六进制和 `0b` 二进制，例如 `0xFF`、`0b1010`。浮点数使用与区域
设置无关的小数点 `.`，并拒绝 `NaN` 和无穷值。

平台参数允许传入裸值或 `名称:值`。当属性声明了默认值时，缺失或空白参数会采用默认值；这对
Config 与 Input 通道完全一致。

## 一维数组

普通数组使用可逆竖线格式：

```text
alpha|beta|gamma
包含\|竖线|包含\\反斜杠
```

竖线和反斜杠必须分别写作 `\|`、`\\`。整型数组还支持递增范围展开，例如 `1-3|0x10` 等价于
`1|2|3|16`；单个范围最多展开 65536 项。

`Vec<u8>` 输出始终为 `base64:...`。输入可以使用同样的 Base64 形式，也可以使用普通整数数组，
例如 `1|2|0xFF`。

## 自定义类型

自定义业务类型实现 `DeviceInput` 与 `DeviceOutput` 后即可用作节点参数和返回值。若它还需要作为
`Vec<T>` 元素，应额外实现 `DeviceCollectionElement`，明确其数组解析和格式化规则。框架不会隐式
将复杂对象数组编码为 JSON，避免在通用 ABI 中引入不可见的数据结构契约。
