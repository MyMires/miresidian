# 文档中心

[项目首页](../README.md) · [架构总览与 UML](architecture.md) · [修订记录](../CHANGELOG.md)

`docs/` 按阅读任务保留六份文档。先选手册，再通过章节导航查阅具体规则；每个主题集中在一个位置维护。

## 六份文档

| 文档 | 内容与阅读对象 |
| --- | --- |
| [本页：文档中心](README.md) | 统一入口、阅读路径、包级说明和文档合并索引 |
| [架构总览](architecture.md) | 中文图注、crate 依赖、UML 类图、调用时序、源码索引与演进计划 |
| [节点开发与数据编解码](宏驱动开发.md) | 节点作者：职责边界、宏标注、参数类型、数组和字节/文本编解码 |
| [平台接入与 C ABI](c-abi.md) | 宿主及平台开发者：C 入口、内存与错误约定、调用契约、平台适配器 |
| [开发与维护规范](coding-standards.md) | 框架维护者：代码与文档要求、构建环境、验证命令、提交格式 |
| [版本管理](versioning.md) | P0 门禁：版本判断、依赖/锁文件、修订归档、发布顺序与标签 |

## 按任务阅读

| 任务 | 建议顺序 |
| --- | --- |
| 第一次了解项目 | [项目首页](../README.md) → [架构总览](architecture.md) |
| 开发设备节点 | [节点声明](宏驱动开发.md#声明方式) → [数据编解码](宏驱动开发.md#数据编解码) → [打包工具](../tools/miresidian-pack/README.md) |
| 接入节点 DLL | [C ABI](c-abi.md#c-abi) → [平台调用契约](c-abi.md#平台调用契约) → [Windows TestFlow](../tools/miresidian-testflow-windows/README.md) |
| 扩展测试平台 | [核心类型关系](architecture.md#核心类型关系uml) → [平台适配器](c-abi.md#平台适配器) |
| 启用资源租约 | [节点开发](宏驱动开发.md) → [租约接入与协议](../lease/README.md) |
| 修改框架或准备发布 | [版本管理](versioning.md) → [开发规范](coding-standards.md) → [构建环境](coding-standards.md#构建环境) → [打包工具](../tools/miresidian-pack/README.md) |

## crate 与开发工具

包级 README 保留在所属目录，供源码浏览、Cargo 打包和 crate 发布使用。

| 包或工具 | 说明入口 |
| --- | --- |
| Miresidian CLI | [统一命令、实际调用对照与整套下载](../tools/miresidian-cli/README.md) |
| `miresidian` | [项目首页与最小节点](../README.md) |
| `miresidian_abi` | [ABI crate](../abi/README.md) · [公共头文件](../abi/api/miresidian.h) |
| `miresidian_codec` | [编解码 crate](../codec/README.md) |
| `miresidian_macro` | [过程宏 crate](../macro/README.md) |
| `miresidian_lease` | [租约接入、v2 协议、配置锁与迁移](../lease/README.md) |
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

- 按本页六个主题扩展现有章节，相关内容优先合并；新增独立文档须说明主题边界并更新索引与引用。
- 图表保留源码标识符，并配中文名称和可见注释；依赖、类型、调用链变化时同步核对架构图。
- 契约规则集中在对应手册，API 精确签名由源码 Rustdoc 维护；交付变更同步[修订记录](../CHANGELOG.md)并按[版本管理](versioning.md)判断影响。
- 仓库强制指令见 [AGENTS.md](../AGENTS.md)，文档和代码要求见[开发与维护规范](coding-standards.md)。

- [实例管理与多通道执行](instances.md)：默认单实例、Config[0] 主键、Handle Config 布局与迁移。

统一工具入口与整套下载安装：[Miresidian CLI](../tools/miresidian-cli/README.md)。原工具保持独立。
