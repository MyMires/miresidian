# Miresidian Lease

[文档中心](../docs/README.md) · [架构总览](../docs/architecture.md)

`miresidian_lease` 定义设备类型无关的资源租约协议：资源键、配置一致性、随机租约 ID、获取、验证和释放。最低 Rust 版本为 **1.89**。

协议不包含串口、TCP、USB、CAN 或其他设备字段；节点宏把其规范化配置编码为不透明字节，业务节点不直接接触租约令牌。

## 节点接入

节点通常只直接依赖 `miresidian`，通过 feature 启用本地 IPC 能力：

```toml
[dependencies]
miresidian = { version = "=0.14.0", features = ["lease"] }
```

在需要租约的节点结构体上声明资源键与配置字段。例如 CAN 节点可声明：

```rust
#[miresidian::node("CAN 通信", lease(
    activation = selectable,
    resource = "channel_name",
    configuration = ["bit_rate", "data_bit_rate"]
))]
struct CanNode;

impl CanNode {
    #[miresidian::operation]
    fn init_device(
        channel_name: String,
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

`lease(...)` 是 `node` 的子配置。`resource` 与 `configuration` 引用 `init_device` 的 Rust 参数名；宏规范化参数并生成租约请求，运行时持有租约、回滚失败初始化并在清理时释放。业务节点不接触 `LeaseRequest`、令牌或租约回调。`selectable` 会发布资源访问方式配置，允许初始化时选择租约或直连；完整标注规则见[宏驱动开发](../docs/宏驱动开发.md)。

根 crate 始终重导出租约通用模型；未启用 `lease` feature 时不启用本地 IPC，未声明租约的节点保持普通调用路径。

## 本地租约 v2

启用 `local-ipc` feature 后，首个调用方创建协调服务；Windows 使用命名管道，Unix 使用 Unix domain socket。服务按用户环境标识隔离，服务重启会生成新的随机代次。每个租约同时绑定租约 ID、服务代次和原始连接。

需要隔离服务会话寿命时，可在启动进程前设置 `MIRESIDIAN_LEASE_SERVICE_NAMESPACE`；同一会话的进程应使用相同值。该选项仅隔离 IPC 服务，资源配置文件锁仍按用户和资源键共享，不会绕过其他命名空间的配置约束。多进程崩溃回归通过子进程环境使用独立命名空间，避免并行验证或真实服务干扰宿主退出场景。

每个客户端在取得租约前先获取资源配置文件锁。相同资源的兼容配置可持有共享文件锁；不兼容配置被拒绝。文件锁由客户端持有，服务宿主退出不会释放其他客户端的锁；客户端进程退出时操作系统回收其锁。

资源文件位于系统临时目录的 `miresidian-lease-v2/<用户标识摘要>/<资源键摘要>.lock`。SHA-256 只用于固定长度文件名；配置比较使用完整字节。锁文件必须保留，**不得在节点运行期间删除或替换文件、清理该临时子目录**，否则会破坏锁文件身份；全部客户端退出后才可清理。不同客户端必须使用相同的用户环境和系统临时目录。

`LocalLease::validate()` 在原连接上校验会话，失败后永久失效，禁止自动重连或重新申请。每个 IPC 请求的读写总截止时间为 2 秒，资源配置锁的竞争等待上限为 5 秒。框架在初始化完成、复用通道和每次业务调用前校验；失效通道只能关闭和重新初始化。已开始的同步设备操作仍由节点自身的超时和取消机制约束。

失效租约继续持有配置锁，直至设备关闭后 `release()` 或析构。显式释放即使 IPC 失败，也会关闭连接并释放配置锁，同时返回错误；重复释放幂等。运行时按“节点关闭及析构 → 租约释放”的顺序清理全部路径。

## 能力边界与升级

租约协调配置兼容性，不共享物理句柄、不提供跨进程设备事务锁。兼容租约不保证同一物理端口可以重复打开，也不保证请求响应的排他性；这些能力由通信实现和操作系统决定。`LeaseState::First` 仅表示当前服务登记表中的首个租约，不能据此判断跨服务代次的物理所有权。

协议版本为 2，端点使用 `miresidian-lease-v2-<用户标识摘要>`。v1 客户端不参与 v2 文件锁协议，因此升级时必须先停止全部 v1 节点进程，再统一启动 v2 节点。C ABI 仍为 v7。

Windows 上承载服务线程的 DLL 使用 `GetModuleHandleExW` 的 `PIN | FROM_ADDRESS` 固定到进程退出。

本 crate 不替代部署侧授权。用户环境标识不等于身份验证；部署仍须配置 Windows 管道 DACL、服务端身份验证与临时目录访问权限。

## 回归验证

`cargo test --workspace --all-targets --all-features --locked` 包含真实多进程回归：终止服务宿主，确认存活客户端仍拒绝不兼容配置、原通道禁止继续调用、关闭释放后可重新初始化。
