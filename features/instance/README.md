# Miresidian Instance

通用实例策略、单实例容器，以及可选的多实例容器与多通道输入调度。最低 Rust 版本为 1.89。

本检出源码为 **0.17.0**，包名及 Rust 导入路径统一为 `miresidian_instance`，源码位于 `features/instance/`，精确依赖 codec 0.18.0。配套根框架 0.33.0 / macro 0.29.0；实例策略、容器和调度行为保持。

本候选尚未发布，不能使用旧包的版本号安装新包。独立节点须在 crates.io 确认新包和配套框架发行后再迁移；旧版项目继续使用 `miresidian_instances` 的对应版本资料。名称替换和依赖示意见[0.33.0 迁移](../../docs/releases/0.33.0.md)。

基础类型及单实例容器始终可用；`runtime` feature 启用多实例容器和顺序、并行、广播调度。不依赖节点 trait、过程宏、平台标记或具体通信实现。节点项目通常通过根 crate 的 `instance` feature 与同名宏配置接入，参见[实例管理](../../docs/instances.md)。

Config 组合键或 Handle 的提取、主键判空、节点构造和清理由根运行时负责；此 crate 管理对象数量、配置策略与结构化输入计划，不打开设备。相同对象的完整方法调用由根运行时的对象锁保护。

自动 Lease 模式的容器项持有共享节点引用，并不各自持有独立业务状态；Direct 模式持有本地节点。共享所有权仍由根运行时及 lease 后端负责，不由本 crate 的对象数量推导物理连接数量。

0.15.0 导出 plan_multi_channel，供同步和异步执行共用参数校验、跳过及广播计划。通用同步 Parallel 使用最多 32 个工作线程；根运行时采用 Future 调度及实例请求门禁。

0.16.0 的 Broadcast 固定执行通道1一次并复制完整输出到有效槽位；其他通道无需同值。
通道1被跳过时整批跳过，通道1解析/执行失败时整批失败，不选择备用通道。
完整输入/输出例子见[广播规范](../../docs/instances.md#broadcast-的执行源与输出规则0160)。

Config 布局不占用平台选择项：平台由 DLL 请求确定。历史 ConfigLayout.platform 字段仅表示执行方式的起始索引；Handle/租约/执行策略的相对顺序保持，详见[Config 布局](../../docs/instances.md#框架-config-布局)。
