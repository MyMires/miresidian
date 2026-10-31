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

子节点项目（含独立烟测夹具）必须仅依赖 crates.io 已发布的框架版本；禁止 path、git 与 patch/replace/paths 等本地覆盖。框架未发布时先发布，不得用本地依赖绕过。框架工作区内部 crate 的 path 不属于子节点依赖。

其他编码、文档和测试规则见 [编码规范](docs/coding-standards.md)。

框架各 crate 与开发工具按各自实际变更独立判断版本，不因框架根 crate 升版而无关跟升。内部精确依赖应匹配被依赖 crate 的实际版本。

## Node skill 同步维护

- 完整的 Node 项目 skill 保存在 `skills/miresidian-node/`，纳入框架版本控制；主文档指导项目开发、验证和交付，工具用法保留在子文档。
- 修改该 skill 的仓库副本或已安装版本时，必须在同次任务中同步另一份完整内容，包括主文档、引用文档、UI 元数据及维护规则；不得只更新其中一份或只复制 `SKILL.md`。
- 具体同步与验证要求见 `skills/miresidian-node/AGENTS.md`。纯 skill 规范修改评估并记录版本影响，不触发无关框架/工具升版或全量构建验证。
