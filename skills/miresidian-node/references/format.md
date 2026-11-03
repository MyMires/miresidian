# 显式格式编解码（框架 0.19.0 起）

根框架 0.19.0 起提供可选 `format` feature，实现位于框架仓库 `features/format/` 的 `miresidian_format` crate。原有 `miresidian_codec` 继续提供默认解析、统一错误和值模型；节点无需直接依赖本 crate。

```toml
miresidian = { version = "=0.19.0", features = ["format"] }
```

以上为源码候选示例；外部节点只采用 crates.io 已发布精确版本。框架更新不自动修改、升级或重新打包已有节点。

format 与 batch 可分别使用：format 决定数据如何编解码，batch 决定业务方法是否按多条记录重复调用。启用 format 不会自动启用 batch，启用 batch 也不要求 format。

## 单独使用 format：标注与槽位

```rust,ignore
#[miresidian::operation("转换")]
#[miresidian::output("接收数据", format(
    formats = [hex, utf8, base64], selector = "接收数据格式"
))]
fn convert(
    &mut self,
    #[miresidian::input("发送数据", format(
        formats = [utf8, hex, base64], selector = "发送数据格式"
    ))]
    data: Vec<u8>,
) -> MiresidianResult<Vec<u8>> {
    Ok(data)
}
```

生成的 Input 顺序为：发送数据、发送数据格式、接收数据格式；Output 只有接收数据。业务方法只接收内部数据。格式选择值由框架消费，在执行业务之前校验。上例每次调用只处理一份数据，不需要 batch。

- `formats` 必填且非空，允许 ascii、gbk、utf8、utf16le、hex、base64；不允许重复格式。
- 第一项为默认格式，选择输入缺失或空白时采用它；非法值报错。
- 单项例如 `format(formats = [hex])` 固定格式，不生成选择输入，也不接受 selector；多项必须声明 selector。
- 不提供 external、default 或单独的 format 选项。
- 输入格式选择紧随对应的数据输入；输出格式选择按输出声明顺序追加到所有方法输入之后。
- 平台选项名为 ASCII、GBK、UTF8、Unicode、HEX、Base64；解析沿用框架格式别名。
- 只支持普通业务 operation 的 Input、Output，不支持生命周期、探测或 Config。
- 输出可逐项写 `#[output("数据", format(...))]`、`#[output("数量")]`，依次对应元组元素。原有单标注多名称语法保持，不能与逐项语法混用。

## 类型与格式转换矩阵

所有内置实现支持上述六种格式，但根据内部类型确定字节契约，绝不把 Rust 内存布局当作数据协议：

| 内部类型 | 规范表示与转换 |
| --- | --- |
| Vec<u8> | 原始字节；字符格式在平台文本与指定字符编码字节之间转换，HEX/Base64 在平台文本与原始字节之间转换 |
| String、char | 规范 UTF-8 文本；字符格式保留文本并严格校验所选编码能否表达它，HEX/Base64 编解码其 UTF-8 字节 |
| 全部基础整数、f32、f64、bool | 规范 UTF-8 文本；输入复用默认标量解析，输出为规范数值或小写 true/false；HEX/Base64 表达这些文本的字节 |
| 数值及布尔 Vec<T>（u8 除外） | 单条记录内空白分隔元素；不使用竖线、不展开范围、不引入 JSON；空文本是空数组 |
| Option<T> | 空字节表示 None，其他值按 T 转换；None 与 Some(空值) 沿用空值表示的不可区分约定 |
| 自定义类型/枚举 | 显式实现 FormatInput、FormatOutput，声明自己的字节契约；默认 RAW_BYTES=false 表示规范 UTF-8 文本，原始字节类型设为 true；作为 batch 输入时还须实现 Clone |

例如 `i32` 的 HEX 输入 `31 32` 表示文本 `12`，得到整数 12；不表示二进制整数 0x3132。Vec<u8> 的同样输入得到两个字节。String 的 GBK 输入 `中文` 得到同样文本；Vec<u8> 的 GBK 输入得到 GBK 字节。

无内置 Vec<String>、嵌套数组或 char 数组实现，避免臆造元素边界。缺少转换 trait 的类型在编译期失败。普通 bool、数值等无需标注 format，默认规则始终可用。

## 可选组合：format + batch

只有需要批量处理时，才同时开启两个 feature：

```toml
miresidian = { version = "=0.19.0", features = ["format", "batch"] }
```

并在需要展开的参数上增加 batch 标记，例如：

```rust,ignore
#[miresidian::input("发送数据", batch, format(formats = [hex]))]
data: Vec<u8>,
```

format 参数同时标记 batch 后，先按转义竖线拆分记录，再逐条解码，每次业务方法仍接收单条数据。

```text
HEX 输入：31 32|44 45
业务调用：[0x31, 0x32]，然后 [0x44, 0x45]
HEX 输出：31 32|44 45
```

输出先逐次编码，再按原 batch 规则按槽位聚合。每段返回数组保持完整，不展平。单条 format 数值数组可输出 `1 2`，批量则为 `1 2|3 4`。

`|` 专用于 format batch 记录；内容中的 `|`、反斜杠由外层 batch 转义为 `\|`、`\\`。先解析外层记录，再按格式解码；非 batch 不增加这层转义。Base64 输出沿用现有 `base64:` 前缀。

空 format batch 输入是一条空记录，`|41` 是两条记录。聚合输出结合请求的批次记录数理解：一个空字符串可以是一条空结果，不得仅凭文本当作零条。未标注 format 的旧数组/输出规则保持不变。

普通 batch 数值参数不需要 format：所有整数支持既有列表和递增范围 `1-5`，f32/f64 支持 `3.14|3.15`、科学计数法及负值，拒绝范围、不会推断步长。按位置对齐、单值广播、顺序执行和首错停止保持不变。

输入转换失败不执行业务；输出转换失败可能发生在业务副作用之后，停止后续记录，不回滚、不重试。所有字符编码严格报错，不替换非法字符。同步和 async 业务使用同样规则。

## 命名迁移

0.19.0 将新增能力的 feature、宏标签与模块统一为 format：启用 `features = ["format"]`，使用 `format(formats = [...])` 及 `miresidian::format::{FormatInput, FormatOutput}`。原来的 codec feature/标签不保留别名；基础 `miresidian_codec` crate、根 `miresidian::codec` 默认解析 API 保持不变。旧节点继续依赖原版本，无需迁移。
