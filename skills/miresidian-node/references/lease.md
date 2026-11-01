# 租约接入、调用与验收

在节点选择 lease 能力时读取。依赖 features 和 activation 选择见 [框架能力与 features](features.md#lease租约与实际设备连接配套)。以节点精确依赖的已发布版本为准；框架源码的 `lease/README.md`、`docs/宏驱动开发.md` 及相关测试用于核对契约，不能把未发布行为直接用于节点。

## 资源身份与兼容性字段

- resource 引用初始化 Config 的 Rust 参数名，取该参数规范化后的文本作为资源键。实例 Handle 和资源键承担不同职责：不同 Handle 可能访问同一资源，不能用随机 Handle 代替真实设备身份以避开冲突。
- 同一物理资源的各调用方应使用一致的资源标识写法；框架不会自动识别端口别名、大小写差异或不同设备路径指向同一硬件。不同类型设备的资源命名也应避免意外碰撞。
- configuration 是至少包含一个初始化 Config 参数名的有序列表。租约比较包含字段名、顺序与规范化文本；跨 DLL 使用同一约定，不能假设数值等价文本或重排字段后仍然兼容。
- 只列出决定资源配置兼容性的字段，并完整覆盖。例如串口需要评估波特率、数据位、校验和停止位；业务请求文本、实例 Handle 不应机械加入。遗漏实际冲突字段会允许不兼容配置通过。

例如以下声明的业务 Config 顺序为 port_name、baud_rate、data_bits、parity、stop_bits，初始化签名也必须按该顺序提供这些 Config 参数：

```rust
#[miresidian::node("串口设备", lease(
    activation = selectable,
    resource = "port_name",
    configuration = ["baud_rate", "data_bits", "parity", "stop_bits"]
))]
struct Device { /* 节点持有的实际串口连接 */ }
```

初始化签名示意，函数体必须按真实设备建立连接并实现失败清理：

```rust
#[miresidian::operation]
fn init_device(
    #[miresidian::config("端口名称")] port_name: String,
    #[miresidian::config("波特率:9600")] baud_rate: u32,
    #[miresidian::config("数据位:8")] data_bits: u8,
    #[miresidian::config("校验:None")] parity: String,
    #[miresidian::config("停止位:1")] stop_bits: String,
) -> miresidian::MiresidianResult<Self>
```

仅将实际设备支持的取值作为默认值。上例没有实现串口通信，不是可交付驱动；close_device 和必要的 Drop 仍须释放真实连接。

## Config 与 TestFlow 调用

先查询 GetInitConfig，按最终 DLL 的元数据组织参数。对上述单实例/selectable 租约声明及默认平台，完整初始化 Config 为：

```json
["COM3", "9600", "8", "None", "1", "Lease", "tb_lv_test_platform"]
```

前五项为业务配置；Lease 是框架追加的资源访问方式，末项是普通入口的平台适配器。业务 init_device 不接收后两项。改用 Direct 只替换资源访问方式这一项；required 或 disabled 声明不发布该项，须移除对应位置，而不是传空字符串占位。

以下流程演示租约初始化与关闭，需替换端口和配置为目标设备实际值；在 InitDevice 与 CloseDevice 之间插入已实现的业务操作，才能验收业务行为：

```text
GetFunctions()
GetInitConfig()
Config("COM3", "9600", "8", "None", "1", "Lease", "tb_lv_test_platform")
InitDevice()
CloseDevice()
```

Direct 路径另起一次初始化/关闭流程，将 Lease 改为 Direct。不要在对象仍存活时只修改选择器就假定原对象的访问路径会切换；先关闭原对象，再按新配置初始化。required 路径没有 Direct 选择器。

组合 instances 时，完整布局为：业务 Config → Handle（handle 关联时）→ 资源访问方式（selectable 租约时）→ 平台适配（普通入口）→ 执行方式（selectable 执行时）。例如在上述业务配置上同时采用 multiple/handle、selectable 租约和 selectable 执行：

```text
Config("COM3", "9600", "8", "None", "1", "device-1", "Lease", "tb_lv_test_platform", "Sequential")
```

显式 invoke_with 已指定平台，应移除平台适配项。框架字段由框架剥离，不添加到业务方法签名；TestFlow 使用普通入口，保留相应平台项。方法参数均为 JSON 字符串，完整工具边界见 [工具参考](commands.md#testflow-写法与执行)。

## 生命周期与失效恢复

运行时在构造设备前取得租约，设备初始化完成后再次验证；复用对象和每次业务调用前也会验证。初始化失败时，尚未返回的局部连接由节点自行回滚；已交给运行时的对象按“关闭并析构节点 → 释放租约”清理。节点不手工申请第二份租约，也不自行持有框架令牌。

服务宿主退出或原连接校验失败后，原租约永久失效；服务重新可用也不会恢复原通道。停止业务调用，执行 CloseDevice 清理后重新 InitDevice；禁止在原通道上自动重连、重新申请租约或盲目重试有副作用的业务。失效租约仍持有配置锁，直到设备清理后释放，避免存活连接被不兼容配置重新打开。

关闭应先停用连接和后台任务，再归还框架控制；即使关闭返回错误，也要尽力释放真实资源。框架释放租约时即使 IPC 出错也会回收本地连接和配置锁，不应把关闭错误当作对象仍可继续使用。已开始的同步设备操作由节点自己的超时/取消机制约束，租约不会强行终止设备 I/O。

租约只协调参与协议的客户端的配置兼容性。它不共享物理连接，不保证兼容配置能重复打开独占串口，也不提供跨进程通信事务锁。LeaseState::First/Shared 不能用于推断物理句柄所有权；选择 Direct 或不参与协议的外部程序也不会受租约约束。

## 本地 IPC v2 的部署要求

- 协调服务由首个调用方自动创建；Windows 使用命名管道，Unix 使用 Unix domain socket，不要求节点自行启动额外服务。
- 参与协调的进程使用相同用户环境和系统临时目录。资源锁位于临时目录下的 miresidian-lease-v2；节点运行期间不得删除、替换锁文件或清理该目录，全部客户端退出后才可清理。
- 需要隔离服务会话寿命时，在启动进程前设置 MIRESIDIAN_LEASE_SERVICE_NAMESPACE，同一会话使用相同值。它仅隔离 IPC 服务，资源配置文件锁仍共享，不能用于绕过配置冲突。
- IPC 单请求读写总截止时间为 2 秒，资源配置锁竞争等待上限为 5 秒；这些是租约协调边界，设备通信另设实际超时。
- 从协议 v1 升级 v2 时，先停止全部 v1 节点进程，再统一启动 v2；二者不共享完整锁协议，不能混用来保证协调。
- 用户环境标识不等于身份认证。交付跨进程访问时，按部署要求配置 Windows 管道 DACL、服务端身份验证及临时目录权限；Windows 承载协调服务的 DLL 会固定到进程退出，不能承诺宿主进程内卸载回收该 DLL。

## 验收场景

只验证节点实际选择的策略和交付范围；普通 Node 开发不重跑整个框架回归。没有真实硬件时，可用独立的无硬件连接夹具验证协调与清理，并明确实际设备打开/通信尚未验证。

| 场景 | 应取得的证据 |
| --- | --- |
| feature 与声明 | 普通构建/交付启用 lease；GetInitConfig 的选择器与 required/disabled/selectable 一致 |
| 兼容配置 | 两个独立客户端对同一资源、同配置可取得租约；真实设备能否重复打开单独判断 |
| 配置冲突 | 一个客户端持有资源时，另一客户端使用不兼容配置被拒绝，且没有先执行业务设备构造 |
| 初始化失败 | 构造失败后局部连接已清理，租约已回滚，后续合法初始化可成功 |
| 正常关闭 | 连接/后台任务已停止，租约释放；全部相关对象关闭后允许重新选择配置 |
| selectable 的 Direct | 新初始化按 Direct 路径运行，不声称参与租约协调；仅在确实支持该选择时验证 |
| 跨进程服务退出 | 存活客户端原通道拒绝业务；不兼容配置仍被配置锁阻止；关闭清理后可重新初始化 |
| instances + lease | Handle/Config 定位正确对象；同资源不能靠换 Handle 绕过配置冲突；关闭一个对象不误清理其他对象 |

冲突是预期失败，应核对对应错误及设备未被构造的证据；不能将任意 BAT 非零退出都当作验收通过。当前 TestFlow 遇到调用失败会终止，预期冲突和多进程服务退出场景应通过相应测试程序或独立进程编排验证，不写成一条预期失败后仍继续执行的普通流程。
