# Miresidian Lease

0.24.0 的参数组合键、默认规则及迁移见 [Config 组合身份与租约声明](../../docs/config-identity.md)。

[中文](README.md) · [English](README.en.md)

[文档中心](../../docs/README.md) · [架构总览](../../docs/architecture.md)

`miresidian_lease` 定义设备类型无关的资源租约协议：资源键、配置一致性、随机租约 ID、获取、验证和释放。最低 Rust 版本为 **1.89**。

协议不包含串口、TCP、USB、CAN 或其他设备字段；节点宏把其规范化配置编码为不透明字节，业务节点不直接接触租约令牌。

## 节点接入

节点通常只直接依赖 `miresidian`，通过 feature 启用本地 IPC 能力。以下为源码候选示例，须待对应版本已在 crates.io 发布后采用：

```toml
[dependencies]
miresidian = { version = "=0.25.0", features = ["lease"] }
```

在结构体上声明租约策略，在初始化 Config 参数上选择资源键；未标记参数自动参与配置比较。例如 CAN 节点可声明：

```rust
#[miresidian::node("CAN 通信", lease(
    activation = selectable
))]
struct CanNode;

impl CanNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("通道名称", lease(key))] channel_name: String,
        bit_rate: u32,
        data_bit_rate: u32,
    ) -> miresidian::MiresidianResult<Self> {
        // 在此建立实际设备连接；示例只演示租约声明。
        let _ = (channel_name, bit_rate, data_bit_rate);
        Ok(Self)
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> {
        Ok(())
    }
}
```

`lease(...)` 是 `node` 的子配置，仅声明 activation。参数 `config("名称", lease(key))` 组成资源键（至少一个），未标记的 Config 自动参与兼容性比较，`lease(ignore)` 排除；多字段按参数顺序生成无歧义组合身份。宏按原 Rust 类型规范化，运行时持有租约、回滚失败初始化并在清理时释放。实例分发独立使用 instance(key/ignore)，不根据租约推导。旧 resource/configuration 字段列表保留兼容，但不能与参数 lease 角色混用。完整规则与迁移见 [组合身份声明](../../docs/config-identity.md)。

根 crate 始终重导出租约通用模型；未启用 `lease` feature 时不启用本地 IPC，未声明租约的节点保持普通调用路径。

## 本地租约 v2

启用 `local-ipc` feature 后，首个调用方创建协调服务；Windows 使用命名管道，Unix 使用 Unix domain socket。服务按用户环境标识隔离，服务重启会生成新的随机代次。每个租约同时绑定租约 ID、服务代次和原始连接。

需要隔离服务会话寿命时，可在启动进程前设置 `MIRESIDIAN_LEASE_SERVICE_NAMESPACE`；同一会话的进程应使用相同值。该选项仅隔离 IPC 服务，资源配置文件锁仍按用户和资源键共享，不会绕过其他命名空间的配置约束。多进程崩溃回归通过子进程环境使用独立命名空间，避免并行验证或真实服务干扰宿主退出场景。

每个客户端在取得租约前先获取资源配置文件锁。相同资源的兼容配置可持有共享文件锁；不兼容配置被拒绝。文件锁由客户端持有，服务宿主退出不会释放其他客户端的锁；客户端进程退出时操作系统回收其锁。

资源文件位于系统临时目录的 `miresidian-lease-v2/<用户标识摘要>/<资源键摘要>.lock`。SHA-256 只用于固定长度文件名；配置比较使用完整字节。锁文件必须保留，**不得在节点运行期间删除或替换文件、清理该临时子目录**，否则会破坏锁文件身份；全部客户端退出后才可清理。不同客户端必须使用相同的用户环境和系统临时目录。

`LocalLease::validate()` 在原连接上校验会话，失败后永久失效，禁止自动重连或重新申请。每个 IPC 请求的读写总截止时间为 2 秒，资源配置锁的竞争等待上限为 5 秒。框架在初始化完成、复用通道和每次业务调用前校验；失效通道只能关闭和重新初始化。已开始的同步设备操作仍由节点自身的超时和取消机制约束。

失效租约继续持有配置锁，直至设备关闭后 `release()` 或析构。显式释放即使 IPC 失败，也会关闭连接并释放配置锁，同时返回错误；重复释放幂等。运行时按“节点关闭及析构 → 租约释放”的顺序清理全部路径。

## 物理资源共享

本 crate 源码候选为 0.17.0，对应根框架 0.25.0。依赖示例须在对应版本已发布后使用；本次源码修改不发布 registry。

根框架保留现有 lease 标注。Lease 模式由 SharedResource 自动持有唯一节点资源对象：第一个引用初始化，后续引用取得相同初始化输出并转发业务调用，最后引用调用 close_device、析构，再释放所有者租约。Direct 模式仍本地初始化。节点状态随资源对象一起共享，不能用此模式获得独立业务计数。

资源键和配置编码一致之外，节点名称及接口、全部规范化业务 Config 和 Init Input 原始文本也必须一致。lease(ignore) 仍排除配置租约比较，但不会允许共享节点以不同初始化状态运行；此类差异明确拒绝。Handle、DLL 文件名和项目版本不参加共享身份。不同业务状态或不同节点类型需要共同通信层，使用 SharedResource 自行持有物理连接，根节点不重复声明资源级 lease。

ResourceHandler::close(request) 只在最后会话退出时调用，随后无论返回错误或 panic 都析构资源并释放配置租约。SharedResource::close_with(request, timeout) 传入关闭参数并返回最后关闭的响应；非最后引用返回空字节，异常断连的关闭请求为空。transact_with_timeout 的 None 等待实际完成，不强加业务超时；Some 沿用 transact 的截止时间和不重放规则。

接入与双 DLL 验证见[共享资源代理](../../docs/shared-resources.md)。

## 能力边界与升级

LocalLease 只协调配置兼容性，不共享物理句柄、不提供跨进程设备事务锁。SharedResource 在此基础上提供唯一连接所有者和完整事务互斥；根框架 0.25.0 的 Lease 标注自动将节点对象接入此路径。兼容租约不保证同一物理端口可以重复打开，也不保证请求响应的排他性；这些能力由通信实现和操作系统决定。`LeaseState::First` 仅表示当前服务登记表中的首个租约，不能据此判断跨服务代次的物理所有权。

协议版本为 2，端点使用 `miresidian-lease-v2-<用户标识摘要>`。v1 客户端不参与 v2 文件锁协议，因此升级时必须先停止全部 v1 节点进程，再统一启动 v2 节点。C ABI 仍为 v7。

Windows 上使用 GetModuleHandleExW 的 PIN | FROM_ADDRESS，Unix 上保留实际承载模块的加载引用，服务及资源回调所在模块保留到进程退出。

本 crate 不替代部署侧授权。用户环境标识不等于身份验证；部署仍须配置 Windows 管道 DACL、服务端身份验证与临时目录访问权限。

## 回归验证

`cargo test --workspace --all-targets --all-features --locked` 包含真实多进程回归：终止服务宿主，确认存活客户端仍拒绝不兼容配置、原通道禁止继续调用、关闭释放后可重新初始化。

资源代理协议独立升级为 v2，端点及所有者锁目录使用 miresidian-resource-v2；关闭帧增加请求与响应。升级前须停止全部 v1 资源代理进程，不能混用。配置租约协议仍为 v2，C ABI v7 不变。
