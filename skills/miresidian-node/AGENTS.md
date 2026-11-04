# Miresidian Node skill 维护规则

仅约束 skill 维护。权威副本为 [MyMires/miresidian](https://github.com/MyMires/miresidian) 的 master 分支 `skills/miresidian-node/`。

- 安装副本通常在 `${CODEX_HOME:-$HOME/.codex}/skills/miresidian-node/` 或 `~/.dsh/skills/miresidian-node/`，以实际位置为准；不硬编码机器路径。
- 修改任一副本时，同次任务同步全部文件及明确删除项，包括引用、UI 元数据和本规则。先比较并合并独有改动，不以旧副本覆盖新内容；另一份不可访问时明确报告未同步。
- 主文档只保留 Node 开发流程、方案确认、完成条件和适配版本。基础规则集中在 references/node-development.md；可选能力按 feature 职责分文件，不按篇幅合并；共用规则归开发参考，工具按需引用，每项规则只定义一次。
- InitDevice、CloseDevice、GetPortNO 分别保留用途、必需/可选、签名、参数来源、返回及资源/失败边界；基础参考覆盖复用、类型空值、输出槽位和异步取消，feature 参考含所需调用/验收示例。框架提供的固定入口说明用途和节点接入，不能仅用方法名或调用示例代替契约。
- instances 与 lease 参考须区分业务对象身份、配置租约和物理资源代理；共享接入明确通信提供者、工厂内打开、完整事务、关闭引用及故障边界，验收不得以租约登记成功代替不同 DLL 的真实加载或实际通信证据。
- 版本表对应真实源码，正文、feature 示例和工具命令只描述表中版本的有效行为；历史沿革和迁移说明留在框架文档与 CHANGELOG。项目版本不匹配时核对该精确版本资料，不混用规则或自动升级。更新时核对 Cargo.toml、ABI/协议常量及实际 API/命令，发布状态另行确认。
- 同步后检查文件清单与内容一致、skill 格式、内部引用/锚点、示例和 UI 元数据；不携带缓存或产物。纯规范变更不重跑无关构建。
- skill 随框架交付，修订归框架 CHANGELOG，遵守仓库版本规则；纯规范变更记录版本影响，不升级无关组件。未经用户同意不引入独立版本或发布流程。
