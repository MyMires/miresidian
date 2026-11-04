# lease：配置租约与共享连接

需要跨 DLL/进程协调资源时读取。适配版本见 [主文档](../SKILL.md#适配版本)：配置租约协议 v2，资源代理协议 v1。业务对象如何复用见 [实例身份](instances.md#对象复用与资源共享)。

## 选择接入路径

| 需求 | 实现与边界 |
| --- | --- |
| 仅检查同资源的连接配置是否兼容 | `lease(...)` 生成请求，运行时持有 LocalLease；每个节点仍自行连接 |
| 不同对象、DLL 或项目共用一个独占连接 | 通信层使用 SharedResource；只打开一次，并互斥执行完整事务 |
| 固定直连 | 通信层自行持有连接；不声称参与共享或配置协调 |

依赖声明：

```toml
[dependencies]
miresidian = { version = "=0.20.1", features = ["lease"] }
```

只开 feature 不会自动加运行时租约，也不会自动代理节点中已有的设备 open。SharedResource 已持有自身及所有者的配置租约，通常由通信层统一负责，无需再声明一套运行时租约。选择路径后落实实际连接、收发和关闭，不能以成功获取租约代替通信实现。

## 配置租约声明

结构体级 lease 的 activation：

| activation | 行为 |
| --- | --- |
| required | 始终取得并验证运行时租约，不追加选择器 |
| disabled | 不取得运行时租约，不追加选择器；保留声明仍需 lease feature |
| selectable | 追加资源访问方式 Config，选择 Lease 或 Direct |

selectable 当前将 Direct/直连解析为直连，空值及其他文本走租约；调用方应传合法值。选择器只控制运行时 LocalLease，不会改变通信对象的实现，也不会把 SharedResource 自动切换为直连。若业务需要在共享连接和直连间选择，由业务 Config 明确声明并由通信层分支；固定共享的节点可省略整个 lease 宏声明，避免两个含义不同的选择器。

- resource 引用初始化 Config 的 Rust 参数名，其规范化文本作为资源键；不是显示标签、Input 或固定值。统一物理资源标识、别名和大小写，不能换 Handle、DLL 名或项目名绕过冲突。
- configuration 至少引用一个 Config 参数名，完整覆盖物理连接兼容性字段。宏编码包含字段名、顺序和规范化文本，跨项目不能只比显示值；Handle 和每次业务 Input 不机械加入。

配置租约的声明与初始化签名片段，假设业务支持所列选项：

```rust
#[miresidian::node("串口节点", lease(
    activation = selectable,
    resource = "port_name",
    configuration = ["baud_rate", "data_bits", "parity", "stop_bits"]
))]
struct SerialNode { /* 持有业务对象 */ }

#[miresidian::operation]
fn init_device(
    #[miresidian::config("端口名称")] port_name: String,
    #[miresidian::config("波特率:9600/19200")] baud_rate: u32,
    #[miresidian::config("数据位:8/7")] data_bits: u8,
    #[miresidian::config("校验:None-无校验/Odd-奇校验/Even-偶校验")] parity: String,
    #[miresidian::config("停止位:1/2")] stop_bits: String,
) -> miresidian::MiresidianResult<Self>
```

选项按 [完整选项规则](node-development.md#完整选项与枚举) 落实；签名片段不包含串口或资源共享实现。

先查询 GetInitConfig。上述单实例/selectable 配置租约的普通入口流程：

```text
GetFunctions()
GetInitConfig()
Config("COM3", "9600", "8", "None", "1", "Lease", "tb_lv_test_platform")
InitDevice()
CloseDevice()
```

前五项为业务 Config，后两项由框架消费。required/disabled 不发布资源访问方式选择器，须移除该项；切换 Lease/Direct 前关闭原对象。组合 instances 或使用 invoke_with 时按 [Config 布局](node-development.md#config-布局) 调整。

## 共享连接接入

通信提供者归 `transport.rs` 或独立复用的通信依赖；node.rs 只声明契约并委托。多个项目共用相同提供者或相同版本的通信事务协议，不将整个节点对象或仪器业务状态移入所有者。

| 接入项 | 必须落实 |
| --- | --- |
| 资源键 ResourceKey | 同一物理连接使用相同的规范化身份；例：serial:COM5，不含 Handle/项目/DLL 名 |
| 配置 LeaseConfiguration | 各项目采用相同的编码与物理参数；波特率、校验等真实兼容性字段完整且无歧义 |
| protocol | 通信层事务线协议及版本，不能将仪器型号或 DLL 文件名当作协议协商 |
| 连接工厂 create | 传给 SharedResource::acquire 的 FnOnce；在工厂内部打开资源并返回 ResourceHandler，不能预先 open 再传入 |
| ResourceHandler | Send + 'static；持有连接，通过 transact(&mut self, request) 执行完整事务并返回字节；析构释放连接 |
| 客户端 SharedResource | 业务对象各自持有代理，通过 transact(request, timeout) 提交请求，关闭时仅 close 自己的代理 |

一次查询的发送、等待、接收必须放在一次 transact 中；拆成两次调用不保留跨调用事务锁。框架不提供内置串口/TCP 命令或通用设备协议；不同项目必须约定请求/响应格式、设备超时、错误与版本兼容规则。不兼容配置或协议应拒绝，不能回退直连重复打开端口。

SharedResource 本身已取得配置租约。若同时声明运行时 lease，双方资源键和配置字节必须完全一致；任意自定义字节不等于宏生成的字段编码。需复用宏编码时，通信提供者可使用同一参数名和顺序的 LeaseRequest::from_parameters 构造请求；避免维护两套不一致的配置定义。

代理是同步接口；在原生 async 业务中通过合适的阻塞线程桥执行并等待实际完成，不能占用异步工作线程或把丢弃等待当作设备动作已取消。通信实现仍负责真实设备 I/O 超时。

可复制的模拟提供者、独占资源动态库示例及完整 API 说明见[对应版本的共享资源文档](https://github.com/MyMires/miresidian/blob/miresidian-v0.20.1/docs/shared-resources.md)。模拟计数器或独占文件证明生命周期，不证明真实串口通信。

## 生命周期与部署

- 运行时配置租约在构造前取得，构造后、复用及业务调用前校验。对象返回前失败由业务回滚；返回后按“关闭并析构对象 → 释放运行时租约”清理。
- 代理工厂只由选出的所有者执行，后续代理不再次打开设备。每个对象仅释放自己的代理，最后代理退出后才析构物理连接。对象复用不累计初始化引用，代理引用独立计数。
- 已有对象的 Lease/Direct 或执行方式不同，重复初始化报错；关闭后重新初始化。工厂失败释放所有权，可重新初始化；处理器 panic 会使该所有者故障，全部旧代理关闭后重建。
- 已发送请求超时或断连使该代理永久失效，不自动重连或重放；未发送前的参数错误、本地排队超时不损坏会话。设备动作可能仍在执行，禁止盲重试有副作用的请求。所有者退出后旧代理失效，显式关闭再初始化。
- 配置租约 IPC 单请求截止 2 秒、配置锁等待上限 5 秒；代理握手等待上限 5 秒，请求/响应各最多 16 MiB。代理事务等待由调用者指定，不替代设备 I/O 超时或配置租约的独立截止时间。
- 参与者使用相同用户环境和临时目录。运行中不得删除或替换 miresidian-lease-v2 的配置锁和 miresidian-resource-v1 的所有者文件；Windows 使用命名管道，Unix 使用本地域套接字，服务模块保留到进程退出。
- MIRESIDIAN_LEASE_SERVICE_NAMESPACE 仅隔离配置协调会话，不隔离物理资源所有者或全局配置锁。部署核对管道 DACL、服务端身份和目录权限；用户环境标识不等于认证。

## 验收

| 场景 | 证据 |
| --- | --- |
| 配置租约 | 同资源同编码兼容，不兼容配置在构造前拒绝；失败/关闭后资源与租约正确释放 |
| 真实共享 | 不同名 DLL、不同节点类型或独立进程附着同一资源，物理工厂只执行一次 |
| 对象身份 | 同 Config 复用；不同 Handle/DLL 保留独立业务状态；换 Handle 不绕过资源约束 |
| 事务与关闭 | 完整收发不交错；关闭一个对象不影响其他对象，最后代理退出才关闭物理连接 |
| 失败 | 工厂失败回滚、设备普通错误、处理器 panic、所有者退出、超时不重放；新初始化与旧会话失效分开验证 |
| 交付范围 | 交付所用 feature、平台和位数的实际 DLL 加载；真实设备通信与模拟结果分别记录 |

核对预期错误与副作用，任意非零退出不算冲突验收通过。TestFlow 遇错终止，预期失败或服务退出场景用测试程序/进程编排。仅取得两份兼容租约不能证明物理连接已共享。
