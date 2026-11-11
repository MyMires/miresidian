# Changelog

## [0.4.0] - 2026-11-11

- 精确依赖 codec 0.17.0，内部转换改用 NodeInput::parse_text、NodeOutput::into_output。
- FormatInput/FormatOutput、格式选择器、槽位和字节契约保持；同步源码适配版本与 README 指导。

## [0.3.2] - 2026-11-05

- 精确依赖 codec 0.16.0，保持现有格式选择和转换接口。
- 验证：相关定向回归、默认/全特性工作区、严格 Clippy、Rust 1.89、文档及 Release 构建通过；完整范围见根 CHANGELOG 0.23.0。

## [0.3.1] - 2026-11-05

- 浮点标量、数组及 Option 输出复用 codec 有限值校验；所有显式编码拒绝 NaN 和正负无穷。
- 精确依赖 codec 0.15.0，采用其数值空白及严格 GBK 解码规则。
- 验证：对应组件与工作区默认/全特性回归、严格 Clippy、Rust 1.89 和 Release 构建通过；完整范围见根 CHANGELOG 0.22.0。

## [0.3.0] - 2026-11-05

- 新增 select_format_value 单条选择校验 API，保持默认/允许列表与字段诊断；不解析批量列表或承担方法调度。
- 与 batch 的组合由宏完成，支持逐条输入/输出编码、单值广播和固定单编码无 selector；保留 decode_formatted_batch 固定一种格式的记录便捷函数以兼容既有调用。
- format 与 batch crate 互不依赖，单独使用 format 仍只选择一个格式，类型/字节转换矩阵不变。

## [0.2.1] - 2026-11-04

- 精确依赖 codec 0.14.2，配合框架统一默认候选文本规范化；格式类型、标注和记录协议不变。
- 验证：默认/全特性工作区回归、严格 Clippy、Rust 1.89、严格 Rustdoc 和 Release 构建通过；与对应框架解析回归一并验证。

## [0.2.0] - 2026-11-03

- 目录统一为 features/format，公开转换 trait 与方法由 CodecInput/CodecOutput、from_codec_bytes/into_codec_bytes 改为 FormatInput/FormatOutput、from_format_bytes/into_format_bytes，包名仍为 miresidian_format。

## [0.1.1] - 2026-11-03

- 说明 codec 与 batch 可独立使用，默认示例仅启用 codec，另列可选批量组合示例；不改变转换实现。

## [0.1.0] - 2026-11-03

- 首版显式格式转换 trait、格式选择及 batch 记录解码；支持原始字节、规范文本、数值/布尔序列和可选值，复用默认 codec 的格式实现。
