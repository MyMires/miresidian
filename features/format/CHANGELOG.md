# Changelog

## [0.2.1] - 2026-11-04

- 精确依赖 codec 0.14.2，配合框架统一默认候选文本规范化；格式类型、标注和记录协议不变。
- 验证：默认/全特性工作区回归、严格 Clippy、Rust 1.89、严格 Rustdoc 和 Release 构建通过；与对应框架解析回归一并验证。

## [0.2.0] - 2026-11-03

- 目录统一为 features/format，公开转换 trait 与方法由 CodecInput/CodecOutput、from_codec_bytes/into_codec_bytes 改为 FormatInput/FormatOutput、from_format_bytes/into_format_bytes，包名仍为 miresidian_format。

## [0.1.1] - 2026-11-03

- 说明 codec 与 batch 可独立使用，默认示例仅启用 codec，另列可选批量组合示例；不改变转换实现。

## [0.1.0] - 2026-11-03

- 首版显式格式转换 trait、格式选择及 batch 记录解码；支持原始字节、规范文本、数值/布尔序列和可选值，复用默认 codec 的格式实现。
