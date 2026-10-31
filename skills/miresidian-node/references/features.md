# 框架能力与 features

从节点需求选择能力，再同时落实依赖 features、节点宏声明、业务实现和验证。下述依赖示例使用已发布的 0.14.0 演示语法；实际保留项目选定的已发布精确版本，不因复制示例降级或升级。不同版本先核对其真实 feature 定义。

## 需求与能力选择

| 节点需要的行为 | miresidian 依赖 features | 节点需要的声明 |
| --- | --- | --- |
| 普通单实例，初始化后执行操作和关闭 | 不额外开启 | 基本 node、operation 等标注 |
| 单实例接受多通道输入 | instances | instances(mode = single, execution = ...) |
| 同时保留多个设备对象 | instances | instances(mode = multiple, binding = config 或 handle, execution = ...) |
| 跨 DLL/进程协调同一资源的配置兼容性 | lease | lease(resource = ..., configuration = ..., activation = ...) |
| 多实例/多通道，并需要资源租约 | instances、lease | 同一 node 上组合 instances(...) 和 lease(...) |

框架根 crate 的 default 为空。节点宏、基本运行时、ABI 和通用编解码不需要另开 macro、runtime、abi 或 codec feature；不要根据模块名猜测 feature。串口/TCP 等具体通信依赖由节点选择，不是框架的同名 feature。

根 feature instances 转发到 miresidian_instances/runtime，lease 转发到 miresidian_lease/local-ipc；普通节点通过 miresidian 开启，不为此额外直接依赖内部 crate。不要把内部 crate 的 feature 名当作根 crate 的 feature。

## 在节点 Cargo.toml 中声明

按实际需要选择一种配置，不能把下面的同名依赖条目叠加粘贴：

```toml
# 默认能力：普通单实例。
[dependencies]
miresidian = { version = "=0.14.0" }
```

```toml
# 需要显式实例策略或多通道。
[dependencies]
miresidian = { version = "=0.14.0", features = ["instances"] }
```

```toml
# 需要资源租约，不要求多实例。
[dependencies]
miresidian = { version = "=0.14.0", features = ["lease"] }
```

```toml
# 两项能力都需要。
[dependencies]
miresidian = { version = "=0.14.0", features = ["instances", "lease"] }
```

这些是框架依赖的 features，不是节点包自己的同名 feature。节点自身写一个空的 instances = [] 不会启用 miresidian/instances。Rust 的 cfg(feature = "...") 判断当前 crate 的 feature，也不能据此判断依赖 feature 是否打开。

如果项目确需将能力作为自身 feature 暴露，显式转发。例如：

```toml
[dependencies]
miresidian = { version = "=0.14.0" }

[features]
default = ["multi-device"]
multi-device = ["miresidian/instances"]
```

示例让普通构建启用 multi-device，配套节点必须有实例策略声明。不要无需求引入这一层转发。确需支持禁用分支时，业务源码和契约也必须支持对应分支；不能仅声明可选开关却让禁用后编译失败。当前 Pack 不传 --features，所需能力应位于依赖固定 features 或有效的节点默认 feature 中，不能只靠一次临时检查启用。

Cargo feature 是加法合并的；default-features = false 只关闭该依赖默认 feature，不会撤销显式 features 或其他依赖路径已启用的能力。普通交付按真实能力配置验证，不把 --all-features 当作节点默认验收方式。

## instances：依赖开关与节点策略配套

打开依赖 feature 只提供能力；未声明 instances(...) 的节点仍是默认单实例，不会自动支持多对象或多通道。

按业务选择一个声明，替换同一节点的结构体标注；下面各段是独立策略示意，不是在一个 crate 声明三个节点：

```rust
// 一个设备对象，多通道输入；初始化时可选执行方式。
#[miresidian::node("设备", instances(
    mode = single,
    execution = selectable,
    default_execution = sequential
))]
struct Device { /* 实际连接与业务状态 */ }
```

```rust
// 多个设备对象，以第一个业务 Config 关联。
#[miresidian::node("设备", instances(
    mode = multiple,
    binding = config,
    execution = selectable,
    default_execution = sequential
))]
struct Device { /* 实际连接与业务状态 */ }
```

```rust
// 多个设备对象，以框架 Handle 关联，固定顺序执行。
#[miresidian::node("设备", instances(
    mode = multiple,
    binding = handle,
    execution = sequential
))]
struct Device { /* 实际连接与业务状态 */ }
```

mode 必填；multiple 还须 binding，single 不接受 binding。execution 必填，可选 sequential、parallel、broadcast、selectable；只有 selectable 才声明 default_execution，并向调用方发布执行方式 Config。

config 关联要求初始化第一个业务 Config 为真实主键；空主键跳过初始化，同键同配置复用，同键不同配置报错。handle 关联由框架在业务 Config 后追加 Handle，业务初始化签名不接收它；空 Handle 初始化跳过，业务调用和关闭不能以空 Handle 选择其他对象。

多个对象与多通道输入是两个维度。一次多通道请求仍按一份 Config 定位同一对象；parallel 不等于同一设备上同时执行多段通信事务。broadcast 的具体副作用和执行源规则应按所选发布版本核对，不按名称猜测。

## lease：租约与实际设备连接配套

打开 lease 后，还要声明资源身份和会影响兼容性的配置字段。例如：

```rust
#[miresidian::node("串口设备", lease(
    activation = selectable,
    resource = "port_name",
    configuration = ["baud_rate"]
))]
struct Device { /* 实际串口连接与业务状态 */ }
```

resource 和 configuration 引用 init_device 的 Rust 参数名，不是显示标签或固定资源值；该例要求初始化签名确有 port_name、baud_rate，并将 port_name 放在第一个业务 Config。实际串口还应声明数据位、校验、停止位等影响配置兼容性的字段，不能机械照抄只填波特率。

selectable 发布资源访问方式，允许初始化选择租约或直连；只开启 feature 而不声明 lease(...) 不会给普通节点自动加租约。运行时负责取得、校验、回滚和释放租约；节点只实现实际设备连接，不手工维护令牌或按 LeaseState 判断物理句柄所有权。

租约只协调配置兼容性，不共享物理连接，也不提供跨进程请求—响应互斥。兼容租约不保证同一串口可被重复打开。业务事务和清理仍由节点负责；租约失效后不得继续使用原通道，应关闭后重新初始化。

## 组合使用及 Config 布局

需要两项能力时，在一个 node 声明中组合，依赖 features 同时开启两项：

```rust
#[miresidian::node("串口设备",
    instances(
        mode = multiple,
        binding = config,
        execution = selectable,
        default_execution = sequential
    ),
    lease(
        activation = selectable,
        resource = "port_name",
        configuration = ["baud_rate"]
    )
)]
struct Device { /* 实际连接与业务状态 */ }
```

以上只展示能力声明，仍需实现 init_device、业务及 close_device。配置参数和初始化资源必须与声明一致。

调用方 Config 按顺序包含：业务 Config → Handle（handle 关联时）→ 资源访问方式（selectable 租约时）→ 平台适配（普通入口）→ 执行方式（selectable 执行时）。显式 invoke_with 不接受平台适配 Config。框架字段在调用业务前剥离，不应写成节点自己的初始化参数。新增能力后重新核对 GetInitConfig 和调用说明，不凭固定参数数量拼接流程。

## 能力对应的验证范围

| 本节点所选能力 | 相应验证 |
| --- | --- |
| 默认单实例 | 本次业务初始化、调用、关闭；不额外测试全部实例策略 |
| multiple/config | 两个主键对象隔离、同键复用/配置冲突、空主键跳过、正确目标关闭 |
| multiple/handle | Handle 元数据、对应对象定位、缺失/未知 Handle、关闭不影响其他对象 |
| 单/多实例多通道 | 仅验证声明的固定策略，或任务要求支持的 selectable 选项；核对输出槽位和相关副作用 |
| lease | 资源键/兼容性字段正确、冲突配置被拒绝、失败/关闭清理；涉及跨进程交付时增加相关进程验证 |
| instances + lease | 所选对象的配置/资源正确关联，关闭对象释放对应连接和租约，其他对象行为符合设计 |

编译配置要与实际交付一致，不能拿仅临时启用 feature 的宿主检查代替正式 DLL。命令见 [工具参考中的 feature 检查](commands.md#feature-配置的检查与构建)。本次若只修改此 skill，检查示例与来源、格式、引用和同步即可，不执行上述节点行为测试。
