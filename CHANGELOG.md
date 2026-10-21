# Changelog

本文记录 Miresidian 框架的已发布修订。格式遵循 Keep a Changelog，版本号遵循语义化版本。

## [Unreleased]

尚未发布的变更必须先记录在本节，发布时移动到对应版本与日期下。

### Added

- `NodeRuntime` 按完整 Config 创建、复用、定位和关闭多个通道对象。
- 支持以 `Input[0] = "共享"` 对全部已创建通道执行自定义操作；空白或参数不符合的通道会跳过。
- `#[operation("名称", parallel)]` 可声明共享调用的并行分发；默认顺序执行。
- 版本标识统一为 `miresidian_version`、`miresidian_abi_version` 与 `miresidian` 发布清单键。

## [0.1.0] - 2026-10-21

### Added

- 建立框架版本、实例项目版本及 C ABI 版本的独立管理规则。
- 增加 `GetFrameworkVersion` 与 `GetProjectVersion` 常驻 ABI 查询。
- `cargo dll` 发布包增加版本、Git 修订和 DLL SHA-256 清单。

### Compatibility

- C ABI: 5
