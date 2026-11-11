# 文档中心

[项目首页](../README.md) · [架构总览与 UML](architecture.md) · [修订记录](../CHANGELOG.md)

本页是主题索引；当前源码版本见 [Cargo.toml](../Cargo.toml)，版本变化见 [CHANGELOG](../CHANGELOG.md)，发行证据见对应归档。专题手册不各自维护“当前版本/已发布版本”表。

## 核心文档与专题

| 分类 | 文档 | 唯一负责的内容 |
| --- | --- | --- |
| 项目结构 | [架构总览](architecture.md) | crate 分层、类型关系、源码位置、执行器设计及后续演进计划 |
| 节点开发 | [节点开发与数据编解码](宏驱动开发.md) | 方法与参数声明、描述及预填规范、类型解析、输出编码 |
| 运行时专题 | [Config 身份](config-identity.md) | instance/lease 参数角色、组合键选择、身份规范化与迁移 |
| 运行时专题 | [实例管理](instances.md) | 对象复用与关闭、Handle、Config 布局和多通道执行 |
| 运行时专题 | [共享资源](shared-resources.md) | 跨 DLL/进程所有者、物理连接、代理事务和引用生命周期 |
| 宿主接入 | [平台接入与 C ABI](c-abi.md) | C 入口、响应与内存契约、TB 文本协议和平台适配器 |
| 仓库维护 | [开发与维护规范](coding-standards.md) | 方案门禁、编码、验证、构建环境和交付组织 |
| 仓库维护 | [版本管理](versioning.md) | 版本判断、依赖/锁文件、修订归档、提交门禁和发布标签 |
| 历史记录 | [0.33.0 名称迁移](releases/0.33.0.md) · [发行说明](releases/) · [维护历史与复盘](history/maintenance.md) · [任务方案](../plans/) | 版本迁移、历史背景、实施过程和验收证据，不作为第二套现行规范 |

## 按任务阅读

| 任务 | 阅读路径 |
| --- | --- |
| 第一次了解项目 | [项目首页](../README.md) → [架构总览](architecture.md) |
| 开发节点插件 | [参数与方法声明](宏驱动开发.md) → 按需查实例/共享资源专题 → [打包工具](../tools/miresidian-pack/README.md) |
| 接入节点 DLL | [C ABI](c-abi.md#c-abi) → [平台调用契约](c-abi.md#平台调用契约) → [TestFlow](../tools/miresidian-testflow-windows/README.md) |
| 多个 DLL 共用独占连接 | [Config 身份](config-identity.md) → [共享资源](shared-resources.md) |
| 修改框架或准备发布 | [开发规范](coding-standards.md) → [版本管理](versioning.md) |
| 使用 AI 开发 Node | [Node skill](../skills/miresidian-node/SKILL.md)：开发流程、资料查询与检查入口 |

## 按实际版本查阅

本检出目录的手册描述本目录源码；源码版本、Git 标签、crates.io 可安装版本和 docs.rs 构建结果分别判断。在线查阅或维护旧项目时：

1. 从目标节点的 Cargo.toml、Cargo.lock 和解析图确定 Node、框架及各组件版本，工具另查实际程序的 --version 和 --help；多个目标包分别核对。
2. 在 [框架发行记录](https://crates.io/crates/miresidian/versions) 确认所需版本，在 [docs.rs](https://docs.rs/miresidian) 切换到精确版本。默认首页或仓库 master 不能代替该版本的契约。
3. 工具分别从 [CLI](https://crates.io/crates/miresidian_cli/versions)、[Pack](https://crates.io/crates/miresidian_pack/versions)、[TestFlow](https://crates.io/crates/miresidian_testflow_windows/versions) 的发行记录进入对应版本 README/源码，不从框架版本推算工具版本，也不照抄开发分支中的候选安装版本。
4. 如需完整仓库手册，从发布包 `.cargo_vcs_info.json` 的 git.sha1 和 path_in_vcs 定位 [官方仓库](https://github.com/MyMires/miresidian) 的实际发布提交；或核实对应组件不可移动标签后，在该快照打开 docs/README.md。工具 README 的相对手册链接在单独发布包中可能没有目标文件，此时沿发布提交查完整仓库。
5. 某些旧版根 API 文档未展示全部可选能力。先核对精确版本的 feature 手册；需要公开类型签名时按解析到的组件版本查组件 Rustdoc/发布源码，或在具备该版本源码的环境执行全特性文档构建。页面缺失不等于该版本没有该能力，也不能据此使用新版签名。

框架 docs.rs 配置使用 all-features，公开的 format 模块须出现在构建结果中。维护时执行 `cargo doc -p miresidian --all-features --no-deps --locked`，核对 `target/doc/miresidian/format/index.html`；跨 crate 的重导出链接可能指向其独立组件文档，宏内部隐藏模块不作为节点公开 API。已发布旧包不可覆盖，修复通过新版本发布，旧版本继续使用对应手册与组件资料。

## crate 与开发工具

包级 README 保留在所属目录，供源码浏览、Cargo 打包和 crate 发布使用。

| 包或工具 | 说明入口 |
| --- | --- |
| Miresidian CLI | [统一命令、实际调用对照与整套下载](../tools/miresidian-cli/README.md) |
| `miresidian` | [项目首页与最小节点](../README.md) |
| `miresidian_abi` | [ABI crate](../abi/README.md) · [公共头文件](../abi/api/miresidian.h) |
| `miresidian_codec` | [编解码 crate](../codec/README.md) |
| `miresidian_macro` | [过程宏 crate](../macro/README.md) |
| `miresidian_format` | [格式选择、类型转换及 batch 组合](../features/format/README.md) |
| `miresidian_batch` | [记录解析、长度对齐及输出聚合](../features/batch/README.md) |
| `miresidian_instance` | [通用策略、容器及多通道调度](../features/instance/README.md) |
| `miresidian_lease` | [租约接入、v2 协议、配置锁与迁移](../features/lease/README.md) |
| `miresidian_pack` | [安装、输入要求与交付目录](../tools/miresidian-pack/README.md) |
| `miresidian_testflow_windows` | [安装、BAT 生成与执行边界](../tools/miresidian-testflow-windows/README.md) |
| 独立工具冒烟夹具 | [节点交付说明](../tools/fixtures/smoke-node/docs/工具冒烟节点.md) · [夹具修订](../tools/fixtures/smoke-node/CHANGELOG.md) |

完整 Rust API 可执行 `cargo doc --workspace --all-features --no-deps --locked` 生成，从 `target/doc/miresidian/index.html` 进入。

## 文档合并索引

以下独立文件已移除，内容迁入对应章节；仓库现行引用已同步更新。

| 原文件 | 当前章节 |
| --- | --- |
| `platform-contract.md` | [平台接入与 C ABI → 平台调用契约](c-abi.md#平台调用契约) |
| `platform-adapters.md` | [平台接入与 C ABI → 平台适配器](c-abi.md#平台适配器) |
| `data-codec.md` | [节点开发 → 数据编解码](宏驱动开发.md#数据编解码) |
| `build-environment.md` | [开发与维护规范 → 构建环境](coding-standards.md#构建环境) |
| `roadmap.md` | [架构总览 → 后续演进计划](architecture.md#后续演进计划) |

## 维护约定

- 先按上表确定主题归属，再修改该主题的正文；其他页面需要关联时使用章节链接，不复制完整规则、例外或版本表。
- README 负责简介与入口；skill 负责工作流程和资料查询；review 负责检查项；技术手册定义行为；CHANGELOG、releases 和 plans 记录变化与证据。示例应符合所属规则，不能形成另一套规则说明。
- 新文件只用于独立主题或历史归档；不为单个小需求增加专题。已有章节迁移时保留入口或修正引用，图中的源码标识符与实际结构保持一致。
- [文档随版本同步](versioning.md#指导性文档与版本同步门禁)是实际受影响内容的同步，不要求把同一需求写进所有文档。
- 仓库强制指令见 [AGENTS.md](../AGENTS.md)，skill 的完整副本同步见其[维护规则](../skills/miresidian-node/AGENTS.md)。
