# Changelog

## [Unreleased]

## [0.17.1] - 2026-11-11

- 公共源码注释改用业务节点、资源及业务操作语义，不将通用状态限定为设备；类型、常量、函数签名与 ABI v10 行为保持。
- 同步 README；不修改头文件或协议编号。

## [0.17.0] - 2026-11-11

- C ABI 升至 v10，唯一业务入口为 miresidian_invoke(const char *request_json)，删除旧三参数入口、invoke_with 及旧 invoke_request 名称。
- 保留独立版本查询；platform 0/省略使用固定默认平台，MIRESIDIAN_PLATFORM_DEFAULT 替代运行时选择常量，Config 不再选择平台。
- 头文件、元数据、Rust 常量、C 函数指针测试及迁移指导同步；响应 JSON、内存归属和稳定平台编号保持。

## [0.16.0] - 2026-11-10

- C ABI 升至 v9，新增 `MIRESIDIAN_PLATFORM_RUNTIME_SELECT = 0`：带平台入口的零值复用 Config 选择；非零平台、函数签名、响应结构与指针有效期保持。
- 同步头文件、Rust 常量/元数据测试、README 及框架迁移说明；源码候选未发布。
- English: C ABI v9 adds `MIRESIDIAN_PLATFORM_RUNTIME_SELECT = 0` for Config selection in both platform-taking entries. Nonzero selection, signatures and response ownership remain unchanged. Update header, metadata consistency tests and guidance; unpublished source candidate.

## [0.15.1] - 2026-11-07

- 修正 README 的发行状态说明，区分已发布 0.15.0 与本次未发布源码，提供明确的精确依赖及文档入口。
- 仅指导文档补丁；C ABI v8、头文件、导出签名、常量和内存契约保持不变。
- 验证：2 项头文件/元数据一致性测试及严格 Rustdoc 通过。

## [0.15.0] - 2026-11-06

- C ABI 升至 v8，新增完整请求导出声明 `miresidian_invoke_request`；保留两个旧调用入口。
- 头文件与 README 明确可选数值 platform、字段/错误规则、三个调用入口共享缓存的所有权与有效期，以及 v7/v8 宿主迁移。
- 同步 ABI 元数据、常量和头文件一致性测试；候选不自动发布，验证见根方案。

## [0.14.0] - 2026-10-30

- ABI 契约与头文件；当前已发布版本，ABI 协议保持 v7。
- 此组件独立管理语义版本，只在实际变化时升版及单独发布；历史统一列车编号不构成联动要求。
