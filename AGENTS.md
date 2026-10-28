# 仓库强制规则

## P0：版本管理先于提交

- 用户指令优先；其余情况下，[版本管理规范](docs/versioning.md) 是本仓库最高优先级的提交门禁。
- 每次相关更新必须在同一提交中提升版本、同步内部精确依赖与所有锁文件、归档带日期的 CHANGELOG、更新当前版本示例。仅写 Unreleased 不满足提交要求。
- 相关更新包括运行时、宏、编解码、ABI、租约、默认值、错误/返回契约、依赖、MSRV、构建配置、发布工具和交付说明。纯测试或纯规范文档变更也必须明确评估版本影响。
- 先确定版本影响，再修改代码；完成验证后提交，随后核对不可移动的版本标签。框架标签为 miresidian-vX.Y.Z。
- 修改公开 Rust API 或破坏 pre-1.0 兼容性时提升次版本；兼容修复提升补丁版本。C ABI、租约协议与 crate 版本分别判断。
- 首次进入检出目录时执行 `git config core.hooksPath .githooks`，启用暂存区版本检查和提交后标签生成；已有自定义钩子须先整合，不能覆盖。
- 提交前执行 `python3 tools/check_versions.py --base HEAD`；暂存后钩子执行同一脚本的 `--staged` 检查。不得绕过门禁或移动既有标签。
- 提交后执行 `python3 tools/check_versions.py --require-tag` 核对版本提交，最终回复必须报告版本、提交、标签和验证结果。

其他编码、文档和测试规则见 [编码规范](docs/coding-standards.md)。
