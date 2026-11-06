# 文档中心

[项目首页](../README.md) · [架构总览与 UML](architecture.md) · [修订记录](../CHANGELOG.md)

本检出为 0.26.1 注释/文档补丁，尚未发布；0.26.0 的正式发行验收已归档，详见[本次勘误与版本边界](releases/0.26.1.md)。

`docs/` 按阅读任务组织核心手册与专题。先选手册，再通过章节导航查阅具体规则；每个主题集中在一个位置维护。

## 核心文档与专题

| 文档 | 内容与阅读对象 |
| --- | --- |
| [本页：文档中心](README.md) | 统一入口、阅读路径、包级说明和文档合并索引 |
| [架构总览](architecture.md) | 中文图注、crate 依赖、UML 类图、调用时序、源码索引与演进计划 |
| [节点开发与数据编解码](宏驱动开发.md) | 节点作者：职责边界、宏标注、参数类型、数组和字节/文本编解码 |
| [平台接入与 C ABI](c-abi.md) | 宿主及平台开发者：C 入口、内存与错误约定、调用契约、平台适配器 |
| [开发与维护规范](coding-standards.md) | 框架维护者：代码与文档要求、构建环境、验证命令、提交格式 |
| [实例管理](instances.md) | 单实例、Config/Handle 身份、复用、多通道与关闭规则 |
| [Config 组合身份](config-identity.md) | 实例键与租约键的独立选择、参数角色及迁移 |
| [共享资源代理](shared-resources.md) | 通信提供者：跨 DLL/项目共用连接、事务协议、所有权与可运行示例 |
| [版本管理](versioning.md) | P0 门禁：版本判断、依赖/锁文件、修订归档、发布顺序与标签 |

## 按任务阅读

| 任务 | 建议顺序 |
| --- | --- |
| 第一次了解项目 | [项目首页](../README.md) → [架构总览](architecture.md) |
| 开发节点插件 | [节点声明](宏驱动开发.md#声明方式) → [数据编解码](宏驱动开发.md#数据编解码) → [打包工具](../tools/miresidian-pack/README.md) |
| 接入节点 DLL | [C ABI](c-abi.md#c-abi) → [平台调用契约](c-abi.md#平台调用契约) → [Windows TestFlow](../tools/miresidian-testflow-windows/README.md) |
| 扩展测试平台 | [核心类型关系](architecture.md#核心类型关系uml) → [平台适配器](c-abi.md#平台适配器) |
| 启用框架可选能力 | [实例管理](instances.md) / [租约](../features/lease/README.md) / [batch crate](../features/batch/README.md) / [format feature](../features/format/README.md) |
| 多个 DLL 共用一个串口或其他独占资源 | [实例身份](instances.md#实例身份与物理资源身份) → [共享连接](shared-resources.md) → [配置租约](../features/lease/README.md) |
| 修改框架或准备发布 | [版本管理](versioning.md) → [开发规范](coding-standards.md) → [构建环境](coding-standards.md#构建环境) → [打包工具](../tools/miresidian-pack/README.md) |

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
| `miresidian_instances` | [通用策略、容器及多通道调度](../features/instances/README.md) |
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

- 按本页主题扩展现有章节，相关内容优先合并；新增独立文档须说明主题边界并更新索引与引用。
- 图表保留源码标识符，并配中文名称和可见注释；依赖、类型、调用链变化时同步核对架构图。
- 契约规则集中在对应手册，API 精确签名由源码 Rustdoc 维护；交付变更同步[修订记录](../CHANGELOG.md)并按[版本管理](versioning.md)判断影响。
- 指导性文档必须随对应组件版本在同一提交和发布源码中更新，涵盖实际受影响的 README、Rustdoc、命令、模板、手册和示例；升版检查与发布后在线核对按 [文档同步门禁](versioning.md#指导性文档与版本同步门禁) 执行。此要求不绑定 Node skill 的更新周期。
- 仓库强制指令见 [AGENTS.md](../AGENTS.md)，文档和代码要求见[开发与维护规范](coding-standards.md)。
