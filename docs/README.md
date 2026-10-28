# 文档中心

[项目首页](../README.md) · [架构总览与 UML](architecture.md) · [修订记录](../CHANGELOG.md)

这里是 Miresidian 的统一文档入口。先按任务选择阅读路径，再通过主题索引查阅具体契约。当前实现以源码和已归档修订为依据；[后续优化备忘录](roadmap.md) 单独记录计划。

## 按任务阅读

| 任务 | 建议顺序 |
| --- | --- |
| 第一次了解项目 | [项目首页](../README.md) → [架构总览](architecture.md) |
| 开发设备节点 | [架构职责边界](architecture.md#职责与源码位置) → [宏驱动开发](宏驱动开发.md) → [数据编解码](data-codec.md) → [打包工具](../tools/miresidian-pack/README.md) |
| 接入节点 DLL | [C ABI](c-abi.md) → [平台调用契约](platform-contract.md) → [Windows TestFlow](../tools/miresidian-testflow-windows/README.md) |
| 扩展测试平台 | [核心类型关系](architecture.md#核心类型关系uml) → [平台适配器](platform-adapters.md) → [C ABI](c-abi.md) |
| 启用资源租约 | [宏驱动开发](宏驱动开发.md) → [租约接入与协议](../lease/README.md) |
| 修改框架或准备发布 | [版本管理](versioning.md) → [编码规范](coding-standards.md) → [构建环境](build-environment.md) → [打包工具](../tools/miresidian-pack/README.md) |

## 主题索引

每个主题指定一份详细说明；其他页面只保留概览、示例或链接，避免同一规则出现多个维护位置。

| 分组 | 文档 | 内容与边界 |
| --- | --- | --- |
| 项目结构 | [架构总览](architecture.md) | crate 依赖、编译期生成、运行时类型、调用时序和源码导航 |
| 节点开发 | [宏驱动开发](宏驱动开发.md) | 节点声明、方法标注、自动命名与生成职责 |
| 平台契约 | [平台调用契约](platform-contract.md) | 固定操作、Config/Input/Output、通道键、共享调用和描述文本 |
| 平台扩展 | [平台适配器](platform-adapters.md) | 适配接口、注册方式、平台编号与选择规则 |
| 跨语言边界 | [C ABI](c-abi.md) | C 签名、指针有效期、响应字段、状态及稳定错误码；头文件源为 [miresidian.h](../abi/api/miresidian.h) |
| 数据格式 | [数据编解码](data-codec.md) | 标量、数组、文本编码与实际值/显示值转换 |
| 资源协调 | [租约说明](../lease/README.md) | feature 与宏接入示例、v2 本地 IPC、配置锁、失效与迁移规则 |
| 环境与验证 | [构建环境](build-environment.md) | MSRV、工具链及工作区验证命令 |
| 工程规范 | [编码规范](coding-standards.md) | 模块结构、错误、安全、测试、文档和提交消息要求 |
| 版本与发布 | [版本管理](versioning.md) | 版本层次、发布顺序、锁文件、修订、钩子和标签门禁 |
| 演进计划 | [后续优化备忘录](roadmap.md) | 尚未发布的工程化事项 |

## crate 与开发工具

包级 README 保留在所属目录，供源码浏览、Cargo 打包及 crate 发布使用；在这里统一索引。

| 包或工具 | 说明入口 |
| --- | --- |
| `miresidian` | [项目首页与最小节点](../README.md) |
| `miresidian_abi` | [ABI crate](../abi/README.md) |
| `miresidian_codec` | [编解码 crate](../codec/README.md) |
| `miresidian_macro` | [过程宏 crate](../macro/README.md) |
| `miresidian_lease` | [租约 crate](../lease/README.md) |
| `miresidian_pack` | [安装、输入要求与交付目录](../tools/miresidian-pack/README.md) |
| `miresidian_testflow_windows` | [安装、BAT 生成与执行边界](../tools/miresidian-testflow-windows/README.md) |
| 独立工具冒烟夹具 | [节点交付说明](../tools/fixtures/smoke-node/docs/工具冒烟节点.md) · [夹具修订](../tools/fixtures/smoke-node/CHANGELOG.md) |

完整 Rust API 可在仓库根目录执行 `cargo doc --workspace --all-features --no-deps --locked` 生成，从 `target/doc/miresidian/index.html` 进入；类型与方法的精确信息由源码 Rustdoc 维护。

## 文档维护方式

- 项目首页负责定位与最小示例，本页负责阅读路径和主题索引；架构图与源码位置统一维护在[架构总览](architecture.md)。
- 新增文档须加入本页，并提供返回文档中心的链接。既有专题路径保持稳定，方便外部链接和节点交付工具继续引用。
- 调整 crate 依赖、核心类型或调用链时，同步核对架构图和源码索引；图中只展示关键关系，精确签名查阅源码与 Rustdoc。
- 改变契约时修改对应专题；修改发布行为时同时更新[修订记录](../CHANGELOG.md)，版本影响依[版本管理规范](versioning.md)判断。
- 仓库强制指令见 [AGENTS.md](../AGENTS.md)，具体编码与文档要求见[编码规范](coding-standards.md)。
