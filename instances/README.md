# Miresidian Instances

通用实例策略、单实例容器，以及可选的多实例容器与多通道输入调度。最低 Rust 版本为 1.89。

```toml
miresidian_instances = "=0.15.0"
```

基础类型及单实例容器始终可用；`runtime` feature 启用多实例容器和顺序、并行、广播调度。不依赖节点 trait、过程宏、平台标记或具体通信实现。节点项目通常通过根 crate 的 `instances` feature 与同名宏配置接入，参见[实例管理](../docs/instances.md)。

Config[0] 或 Handle 的提取、主键判空、节点构造和清理由根运行时负责；此 crate 管理对象数量、配置策略与结构化输入计划，不打开设备。相同对象的完整方法调用由根运行时的对象锁保护。

0.15.0 导出 plan_multi_channel，供同步和异步执行共用参数校验、跳过及广播计划。通用同步 Parallel 使用最多 32 个工作线程；根运行时采用 Future 调度及实例请求门禁。
