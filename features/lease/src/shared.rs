//! 跨 DLL/进程共享一个物理资源。各节点保留自己的业务对象，通信层通过字节事务代理访问资源。
//!
//! 所有参与者必须约定相同的资源键、配置编码和事务协议；工厂只在选出的资源所有者中执行。
//! 一次 `transact` 应包含完整请求和响应，不能拆成两次调用来期望排他性。
mod client;
mod owner;
mod protocol;
mod server;

pub use client::SharedResource;

/// 共享资源代理的独立线协议版本，不改变配置租约 v2。
pub const RESOURCE_PROTOCOL_VERSION: u16 = 1;
/// 单次资源请求或响应的最大字节数。
pub const MAX_TRANSACTION_BYTES: usize = 16 * 1024 * 1024;

/// 由资源所有者持有的物理连接。协议解析、超时和设备 I/O 仍由通信实现负责。
pub trait ResourceHandler: Send + 'static {
    /// 在资源级互斥范围内执行完整事务；错误不会自动重试或回滚已完成的 I/O。
    fn transact(&mut self, request: &[u8]) -> Result<Vec<u8>, crate::LeaseError>;
}

impl<F> ResourceHandler for F
where
    F: FnMut(&[u8]) -> Result<Vec<u8>, crate::LeaseError> + Send + 'static,
{
    fn transact(&mut self, request: &[u8]) -> Result<Vec<u8>, crate::LeaseError> {
        self(request)
    }
}
