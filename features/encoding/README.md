# encoding：显式格式编解码增强

根框架 0.34.0 将原 format 能力统一更名为可选 `encoding` feature，实现位于本目录的 `miresidian_encoding` crate。原有 `miresidian_codec` 继续提供默认解析、统一错误和值模型；节点无需直接依赖本 crate。

本目录源码候选为 **0.5.1**，精确依赖 codec **0.18.0**，由框架 **0.34.1** 接入。内部转换使用 NodeInput::parse_text、NodeOutput::into_output；EncodingInput/EncodingOutput、字节契约及框架格式选择器保持。

上述版本尚未发布；外部节点只采用 crates.io 已发布精确版本并启用根框架的 `encoding` feature。框架更新不自动修改、升级或重新打包已有节点。

0.5.1 仅修正 crate/转换模块 Rustdoc 中遗留的旧能力名称，准确说明根框架通过 encoding feature 接入；公开 API 与转换行为保持。

encoding 与 batch 可分别使用：encoding 决定数据如何编解码，batch 决定业务方法是否按多条记录重复调用。启用 encoding 不会自动启用 batch，启用 batch 也不要求 encoding。

## 单独使用 encoding：标注与槽位

```rust,ignore
#[miresidian::operation("转换")]
#[miresidian::output("接收数据", encoding(
    hex, utf8, base64
))]
fn convert(
    &mut self,
    #[miresidian::input("发送数据", encoding(
        "发送数据格式", utf8, hex, base64
    ))]
    data: Vec<u8>,
) -> MiresidianResult<Vec<u8>> {
    Ok(data)
}
```

生成的 Input 顺序为：发送数据、发送数据格式、接收数据格式；Output 只有接收数据。业务方法只接收内部数据。格式选择值由框架消费，在执行业务之前校验。上例每次调用只处理一份数据，不需要 batch。

- 格式列表非空，允许 ascii、gbk、utf8、utf16le、hex、base64；不允许重复格式。可直接列出标识符，或采用 formats = [...] 长写。
- 第一项为默认格式，选择输入缺失或空白时采用它；非法值报错。
- 单项例如 `encoding(formats = [hex])` 固定格式，不生成选择输入，也不接受 selector；多项未命名时自动使用外层显示名（去除冒号后的描述并 trim）加“格式”。
- encoding("编码方式", utf8, hex) 用字符串首项命名；原 selector = "..." 仍支持，不能重复或混合两种命名。名称不得为空白或包含 : / |；与业务 Input 或其他格式选择器重名时编译报错，需显式使用不同名称，不自动编号。
- 单格式禁止显式名称，字符串名称必须置首且至少有两种格式；空格式、未知/重复格式与混用位置列表和 formats 数组均报错。不提供 external、default 或自动探测。
- 输入格式选择紧随对应的数据输入；输出格式选择按输出声明顺序追加到所有方法输入之后。
- 平台选项名为 ASCII、GBK、UTF8、Unicode、HEX、Base64；解析沿用框架格式别名。
- 只支持普通业务 operation 的 Input、Output，不支持生命周期、探测或 Config。
- 输出可逐项写 `#[output("数据", encoding(...))]`、`#[output("数量")]`，依次对应元组元素。原有单标注多名称语法保持，不能与逐项语法混用。

## 类型与格式转换矩阵

所有内置实现支持上述六种格式，但根据内部类型确定字节契约，绝不把 Rust 内存布局当作数据协议：

| 内部类型 | 规范表示与转换 |
| --- | --- |
| Vec<u8> | 原始字节；字符格式在平台文本与指定字符编码字节之间转换，HEX/Base64 在平台文本与原始字节之间转换 |
| String、char | 规范 UTF-8 文本；字符格式保留文本并严格校验所选编码能否表达它，HEX/Base64 编解码其 UTF-8 字节 |
| 全部基础整数、f32、f64、bool | 规范 UTF-8 文本；输入复用默认标量解析，输出为有限数值或小写 true/false；HEX/Base64 表达这些文本的字节 |
| 数值及布尔 Vec<T>（u8 除外） | 单条记录内空白分隔元素；不使用竖线、不展开范围、不引入 JSON；空文本是空数组 |
| Option<T> | 空字节表示 None，其他值按 T 转换；None 与 Some(空值) 沿用空值表示的不可区分约定 |
| 自定义类型/枚举 | 显式实现 EncodingInput、EncodingOutput，声明自己的字节契约；默认 RAW_BYTES=false 表示规范 UTF-8 文本，原始字节类型设为 true；作为 batch 输入时还须实现 Clone |

例如 `i32` 的 HEX 输入 `31 32` 表示文本 `12`，得到整数 12；不表示二进制整数 0x3132。Vec<u8> 的同样输入得到两个字节。String 的 GBK 输入 `中文` 得到同样文本；Vec<u8> 的 GBK 输入得到 GBK 字节。

无内置 Vec<String>、嵌套数组或 char 数组实现，避免臆造元素边界。缺少转换 trait 的类型在编译期失败。普通 bool、数值等无需标注 encoding，默认规则始终可用。数值标量空白为零，NaN/正负无穷输入和输出均报错；业务描述不提供运行时默认值，格式选择器仍按允许列表首项显式缺省。

## 可选组合：encoding + batch

只有需要批量处理时，才同时开启两个 feature：

```toml
# 仅示意发布后的迁移目标；0.34.1 候选尚未发布，不可直接安装。
miresidian = { version = "=0.34.1", features = ["encoding", "batch"] }
```

并在需要展开的参数上增加 batch 标记，例如：

```rust,ignore
#[miresidian::input("发送数据", batch, encoding(formats = [hex]))]
data: Vec<u8>,
```

encoding 参数同时标记 batch 后，宏分别拆分数据列和格式列，按长度一广播、其他等长规则对齐，再逐条调用单条 encoding 解码；原方法类型决定每条值。每个编码记录对应一个业务值，解码后的分隔符或整数范围文本不重新展开。

```text
HEX 输入：31 32|44 45
业务调用：[0x31, 0x32]，然后 [0x44, 0x45]
HEX 输出：31 32|44 45
```

### 逐条格式与固定格式

```rust,ignore
#[miresidian::input("文本", batch, encoding(formats = [utf8, hex, base64], selector = "文本输入类型"))]
text: String,
```

`文本=A|42|Qw==` 与 `文本输入类型=UTF8|HEX|Base64` 依次得到 A、B、C。单个格式会广播给全部数据；单个数据也可广播给格式列表，例如 `41` 与 `UTF8|HEX` 得到 41、A。格式空项使用允许列表首项，未知/不允许的值或不等长组合在业务执行前拒绝。

只有数据参数标记 batch 时，它的输入格式选择器才接受列表；没有 batch 的 encoding 仍接受一个格式。batch 方法的输出格式选择器也支持单值广播或等长列表，仍追加在全部方法 Input 之后，按每次调用选择输出编码。

单编码如 `encoding(formats = [hex])` 不接受 selector，也不生成额外输入。`#[input("文本", batch, encoding(formats = [hex]))] text: String` 只需传 `41|42`，两次调用分别得到 A、B；不组合 batch 时同样无需选择器。

batch 与 encoding crate 互不依赖，共同复用 codec，由宏组合。公开函数 select_encoding_value 只校验一条格式值；兼容函数 decode_encoded_batch 只用一种固定格式解码多条记录，不参与批次长度或方法调度。

输出先逐次编码，再按原 batch 规则按槽位聚合。每段返回数组保持完整，不展平。单条 encoding 数值数组可输出 `1 2`，批量则为 `1 2|3 4`。

`|` 专用于 encoding batch 记录；内容中的 `|`、反斜杠由外层 batch 转义为 `\|`、`\\`。先解析外层记录，再按格式解码；非 batch 不增加这层转义。Base64 输出沿用现有 `base64:` 前缀。

空 encoding batch 输入是一条空记录，`|41` 是两条记录。聚合输出结合请求的批次记录数理解：一个空字符串可以是一条空结果，不得仅凭文本当作零条。未标注 encoding 的旧数组/输出规则保持不变。

普通 batch 按原方法类型解析：字符串、布尔、字符、整数和浮点均不需要 encoding。整数保留 `1-3|5|8-10` 混合展开；字符串中的 `1-3` 保持文本，布尔/浮点不推断范围。类型别名和完整路径按相同 codec trait 处理。

输入转换失败不执行业务；输出转换失败可能发生在业务副作用之后，停止后续记录，不回滚、不重试。所有字符编码严格报错，不替换非法字符。同步和 async 业务使用同样规则。

## 0.34.0 命名迁移

原 miresidian_format 0.4.1 迁移为 miresidian_encoding 0.5.0，目录为 features/encoding，根模块为 miresidian::encoding。Cargo feature 和宏分组均为 encoding，不保留旧入口别名。EncodingInput/EncodingOutput、from_encoding_bytes/into_encoding_bytes、decode_encoded/decode_encoded_batch/encode_encoded 和 select_encoding/select_encoding_value 是当前公开名称；转换字节契约与行为保持。

基础 miresidian_codec、miresidian::codec、ByteTextFormat 及默认解析函数保持原名，Rust format! 不受影响。旧版本 Node 继续使用对应版本的 format 用法；采用新版本必须先核实发行并整体迁移。自动命名会随参数显示名称变化，需要固定契约时显式命名选择器。完整映射和边界见 [0.34.0 迁移](../../docs/releases/0.34.0.md)。
