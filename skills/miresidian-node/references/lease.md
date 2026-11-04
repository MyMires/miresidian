# lease：资源配置租约

选择 lease 时读取。适配 [主文档基线](../SKILL.md#适配版本) 的本地 IPC v2；旧版本按其契约核对。租约协调参与者的资源配置兼容性，不共享物理连接，也不提供跨进程通信事务锁；兼容租约不保证独占串口可重复打开。

## 声明与资源身份

依赖示例对应当前源码基线，使用前须确认已发布：

```toml
[dependencies]
miresidian = { version = "=0.19.4", features = ["lease"] }
```

节点声明资源键、兼容性字段及激活策略：

| activation | 行为 |
| --- | --- |
| required | 始终取得并验证租约，不追加选择器 |
| disabled | 固定直连，不追加选择器；保留声明仍需 lease feature |
| selectable | 追加资源访问方式 Config，初始化显式选择 Lease 或 Direct |

普通直连可省略整个 lease 声明；只开 feature 不会自动加租约。selectable 当前将 Direct/直连解析为直连，空值及其他文本走租约；明确传合法值。Direct 不受配置协调约束，需要全部参与者协调时使用 required。

- resource 引用初始化 Config 的 Rust 参数名，其规范化文本作为资源键；不是显示标签、Input 或固定资源值。所有调用方统一物理资源标识，框架不识别别名/大小写；不能换 Handle 绕过冲突。
- configuration 至少一个 Config 参数名，完整覆盖实际兼容性字段；字段名、顺序及规范化文本须跨 DLL 一致，不能假设数值等价文本相同。业务输入和 Handle 不机械加入。

声明与初始化签名片段，假设示例业务确实支持所列全部选项：

```rust
#[miresidian::node("串口节点", lease(
    activation = selectable,
    resource = "port_name",
    configuration = ["baud_rate", "data_bits", "parity", "stop_bits"]
))]
struct SerialNode { /* 持有串口业务对象 */ }

#[miresidian::operation]
fn init_device(
    #[miresidian::config("端口名称")] port_name: String,
    #[miresidian::config("波特率:9600/19200")] baud_rate: u32,
    #[miresidian::config("数据位:8/7")] data_bits: u8,
    #[miresidian::config("校验:None-无校验/Odd-奇校验/Even-偶校验")] parity: String,
    #[miresidian::config("停止位:1/2")] stop_bits: String,
) -> miresidian::MiresidianResult<Self>
```

选项和业务枚举按 [完整选项规则](node-development.md#完整选项与枚举) 落实。实际初始化委托业务对象连接并清理失败资源，关闭委托其释放；示意不含通信实现。

## 调用布局

先查询 GetInitConfig。上述单实例/selectable 租约的普通入口流程如下；按目标资源替换配置，并在初始化与关闭间插入实际业务操作：

```text
GetFunctions()
GetInitConfig()
Config("COM3", "9600", "8", "None", "1", "Lease", "tb_lv_test_platform")
InitDevice()
CloseDevice()
```

前五项是业务 Config，后两项由框架消费。Direct 流程关闭原对象后重新初始化，只替换选择器；required/disabled 不发布选择器，须移除该项。组合实例/多通道或使用 invoke_with 时按 [Config 布局](node-development.md#config-布局) 调整。

TestFlow 普通入口保留平台项，参数均为 JSON 字符串；执行边界见 [工具命令](commands.md#testflow)。

## 生命周期与部署

- 运行时在构造前取得租约、构造后再次验证，复用及业务调用前也验证。局部连接失败由业务对象回滚；交给运行时的对象按“关闭并析构 → 释放租约”清理。节点不另行申请令牌，不按 LeaseState 推断物理句柄所有权。
- 服务宿主退出或连接校验失败后，原租约永久失效；停止业务，CloseDevice 清理后重新 InitDevice，不在原通道自动重连或盲重试。失效租约保留配置锁至对象清理；IPC 关闭错误不代表对象仍可用，已开始 I/O 仍由业务超时/取消机制约束。
- 首调用方自动建立服务：Windows 命名管道，Unix domain socket。参与进程须使用相同用户环境和临时目录；运行期间不得清理临时目录下 `miresidian-lease-v2` 的锁文件。
- `MIRESIDIAN_LEASE_SERVICE_NAMESPACE` 在启动前设置，用于隔离服务会话，资源文件锁仍共享。IPC 单请求截止 2 秒、配置锁竞争上限 5 秒；资源通信单独设超时。
- v1 迁移 v2 前停止全部旧协议进程，混用不能保证协调。跨进程部署核对管道 DACL、服务端身份及临时目录权限；用户环境标识不是认证。Windows 服务宿主 DLL 固定到进程退出。

## 验收

| 场景 | 证据 |
| --- | --- |
| feature/策略 | 正常交付启用 lease，元数据与所选 activation 一致 |
| 兼容/冲突 | 独立客户端同资源同配置取得租约；不兼容配置在业务对象构造前被拒绝 |
| 初始化失败/关闭 | 局部连接、后台任务与租约清理；全部对象关闭后可重新配置 |
| Direct（选择此策略时） | 新初始化走直连，不声称参与协调 |
| 跨进程服务退出 | 原通道拒绝业务，配置锁仍阻止冲突；关闭后可重新初始化 |
| instances 组合 | Handle/Config 定位及关闭隔离正确，换 Handle 不绕过资源冲突 |

核对预期错误和副作用，任意非零退出不算冲突验收通过。TestFlow 遇错终止，预期失败/服务退出场景用测试程序或独立进程编排。无硬件可验证协调与清理，实际连接/通信另行说明。
