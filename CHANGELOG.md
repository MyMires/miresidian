# Changelog

本文记录 Miresidian 框架的已发布修订。格式遵循 Keep a Changelog，版本号遵循语义化版本。

## [Unreleased]

尚未发布的变更必须先记录在本节，发布时移动到对应版本与日期下。

## [0.1.0] - 2026-10-21

### Added

- 建立框架版本、实例项目版本及 C ABI 版本的独立管理规则。
- 增加 `GetFrameworkVersion` 与 `GetProjectVersion` 常驻 ABI 查询。
- `cargo dll` 发布包增加版本、Git 修订和 DLL SHA-256 清单。

### Compatibility

- C ABI: 5
