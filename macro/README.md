# miresidian_macro

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_macro` 是 Miresidian 的过程宏实现 crate，负责生成节点契约、调用分发、生命周期调度和 C ABI 入口。

节点项目应依赖并使用 `miresidian` 重导出的宏，而不需要直接依赖本 crate：

```rust
use miresidian::{config, input, node, operation, output};
```

宏标注和节点实现约束见[宏驱动开发](../docs/宏驱动开发.md)。

`node` 保留在结构体上，生成节点身份、注册绑定和 C ABI；每个 `operation` 独立生成方法契约，支持同模块多个固有 impl。初始化、关闭和可选探测同样使用 operation，节点不再需要 Default。

0.15.0 支持 operation 标注 async 方法，生成原生业务 Future 桥；构造、关闭和探测的 async 桥由生命周期线程等待。同步 DLL 入口共享内部并发运行时。参见[异步方法](../docs/宏驱动开发.md#异步方法)。


## 类型驱动的 batch 与 format 组合

普通业务 Input 的 batch 不再按 Rust 类型名称限制为数值；生成代码使用原参数类型的 DeviceInput、DeviceCollectionElement 与 Clone。整数保留单值/列表/范围/混合，字符串、布尔、字符和别名使用其类型解析。format 的单条编解码独立于 batch；组合宏对齐数据列和格式列，支持格式广播/逐条选择及输出格式列。固定单编码不生成选择输入，原有参数顺序与 metadata 布局保持。

宏只组合各功能 API；batch 与 format 两个实现 crate 不互相依赖。输入解析/选择/长度校验先于业务，保持顺序、首错停止、逐槽输出聚合和同步/async 桥接。

0.20.0 起，业务 Config/Input 描述仅原样发布，不生成运行时默认值；缺参报错，数值显式空白为零，具体选项显示值仍映射。format 选择器等框架生成的显式默认值继续生效。

0.21.0 按初始化 Config 的原 Rust 类型生成归一化回调，根运行时和 lease 参数共用；类型别名通过 trait 分派，不按类型名称猜测。

0.22.0 支持初始化 Config 的 instance(key/ignore)、lease(key/ignore) 嵌套角色；生成独立的实例键索引及租约字段选择，不新增独立参数属性。节点策略仍由 node 子配置声明。参见 [组合身份与迁移](../docs/config-identity.md)。

## 完整请求入口（源码候选 0.23.0）

`node` 自动生成 C ABI v8 的 `miresidian_invoke_request(const char *request_json)`，同时保留原有 `miresidian_invoke`、`miresidian_invoke_with` 及 ABI 版本查询。完整请求含 function、config、input 和可选数值 platform；省略 platform 沿用初始化 Config 的平台选择，提供时沿用显式平台规则。三个调用入口共用同一个运行时，节点作者无需增加标注或手写导出。请求解析和响应生命周期由框架 0.26.0 统一实现，完整字段与迁移见 [C ABI](../docs/c-abi.md#完整请求入口v8)。

本任务未发布宏或框架候选；独立节点只能在所需组件已发布后升级框架精确依赖并重新构建，不能用本地覆盖绕过发布。

当前 Pack 0.13.2 的 `new/init` 模板流程明确只接受 ABI v7，不能用它创建目标 ABI v8 的节点；本任务未升级或验证该模板流程。既有节点在候选发布后手动升级时须按新头文件更新交付说明。TestFlow 保留原同步入口调用方式，不能因此推断其已采用完整请求入口。
