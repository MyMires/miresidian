# Changelog

## [Unreleased]

## [0.3.2] - 2026-11-12

- README 将与实例管理组合的能力名同步为 instance，标明配套框架 0.33.0；仅指导修订，批量解析、聚合、API 和依赖保持。未发布补丁。
- English: Update composition guidance to the instance name for framework 0.33.0. No code or behavior change; unpublished documentation patch.

## [0.3.1] - 2026-11-11

- codec 精确依赖同步至 0.18.0；支持其派生枚举集合契约，记录规则、长度对齐和调用语义保持。

## [0.3.0] - 2026-11-11

- 精确依赖 codec 0.17.0，公开批量解析约束使用 NodeInput/NodeCollectionElement，输出聚合使用 NodeOutput。
- 记录转义、长度对齐、广播、首错停止及输出槽位保持；同步 README 通用业务示例。

## [0.2.2] - 2026-11-05

- 精确依赖 codec 0.16.0，采用转义边界正确的选项显示值映射。
- 验证：相关定向回归、默认/全特性工作区、严格 Clippy、Rust 1.89、文档及 Release 构建通过；完整范围见根 CHANGELOG 0.23.0。

## [0.2.1] - 2026-11-05

- 精确依赖 codec 0.15.0；数值空记录按原标量类型解析为零，普通 Vec 的空集合语义与转义规则不变。
- 验证：对应组件与工作区默认/全特性回归、严格 Clippy、Rust 1.89 和 Release 构建通过；完整范围见根 CHANGELOG 0.22.0。

## [0.2.0] - 2026-11-05

- 新增 decode_batch<T>，复用原类型的 DeviceInput/DeviceCollectionElement；整数保留混合范围，字符串、布尔、字符和类型别名使用对应语法，不依赖 format。
- 新增只拆分转义记录的 batch_records，空文本表示一条记录；长度对齐与输出聚合保持通用无状态，不引入 format 依赖。
- 普通 Vec 的空集合行为不变，非法记录/长度先于方法执行。宏负责与格式列组合，批次输出仍逐槽可逆聚合。

## [0.1.2] - 2026-11-04

- 精确依赖 codec 0.14.2，配合框架统一默认候选文本规范化；批量对齐、顺序执行和聚合契约不变。
- 验证：默认/全特性工作区回归、严格 Clippy、Rust 1.89、严格 Rustdoc 和 Release 构建通过；与对应框架解析回归一并验证。

## [0.1.1] - 2026-11-03

- 同步 codec 精确依赖至 0.14.1，批量执行行为不变。

## [0.1.0] - 2026-11-02

- 首次发布参数级顺序批量调用的公共无状态支持：整数数组长度对齐、单值广播与逐输出槽位聚合。
