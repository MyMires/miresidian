# Changelog

## [Unreleased]

## [0.17.0] - 2026-11-11

- 公开类型 DeviceInput/DeviceOutput/DeviceCollectionElement/DeviceScalar/DeviceValue 分别更名为 NodeInput/NodeOutput/NodeCollectionElement/NodeScalar/NodeValue，不保留别名。
- 关联方法统一为 parse_text、normalize_text、into_output、parse_collection、format_collection；输入、输出、集合和规范化行为保持。
- 同步 Rustdoc、README 自定义业务类型示例和组件回归；验证见根任务方案。

## [0.16.0] - 2026-11-05

- DeviceInput 新增带默认实现的 normalize_device_text，内置数值、bool、格式、Option/Vec 提供规范文本；String 与自定义类型默认保留文本。
- 选项映射按已识别转义的完整集合元素处理，避免字面竖线后半段被误映射。
- 验证：相关定向回归、默认/全特性工作区、严格 Clippy、Rust 1.89、文档及 Release 构建通过；完整范围见根 CHANGELOG 0.23.0。

## [0.15.0] - 2026-11-05

- 删除完整候选文本取首项，保留显式 helper 默认值与实际值/显示值映射；整数和浮点显式空白解析为零，缺参不隐式补值。
- GBK 解码禁用 BOM 探测及非法字符替换，非法字节返回 INVALID_ARGUMENT。
- 验证：对应组件与工作区默认/全特性回归、严格 Clippy、Rust 1.89 和 Release 构建通过；完整范围见根 CHANGELOG 0.22.0。

## [0.14.2] - 2026-11-04

- 仅当当前值与完整候选默认文本一致时取首项实际值；显式候选值、显示值及其他输入独立解析，不回退首项。保留默认文本元数据及无默认值时的缺失错误，并修复显式 default 被选项映射忽略的问题。
- 验证：默认/全特性工作区回归、严格 Clippy、Rust 1.89、严格 Rustdoc 和 Release 构建通过；与对应框架解析回归一并验证。

## [0.14.1] - 2026-11-03

- `ByteTextFormat` 接受 `UTF_8`（大小写不敏感），保留原有 `UTF-8`、`UTF8` 输入及显示值。

## [0.14.0] - 2026-10-30

- 通用编解码与错误模型；当前已发布版本。
- 此组件独立管理语义版本，只在实际变化时升版及单独发布；历史统一列车编号不构成联动要求。
