//! 客户端只持有代理会话；超时/断连后不重连、不重放事务。
use super::{
    MAX_TRANSACTION_BYTES, RESOURCE_PROTOCOL_VERSION, ResourceHandler, owner, protocol::*,
};
use crate::local_ipc::protocol::{io_error, write_bytes, write_u8};
use crate::local_ipc::{LocalLease, client::Connection};
use crate::{LeaseConfiguration, LeaseError, ResourceKey};
use std::{
    io::Write,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

/// 一个独立调用者对共享物理资源的引用。关闭该引用不会关闭其他调用者的连接。
///
/// 不同项目须使用同一个通信层事务协议；此类型不解析仪器业务协议。
pub struct SharedResource {
    connection: Mutex<Option<Connection>>,
    lease: LocalLease,
}

impl SharedResource {
    /// 附着到已有资源，或选举唯一所有者并执行工厂打开资源。
    ///
    /// `protocol` 是通信层线协议及版本标识；兼容配置但协议不同也会拒绝。
    /// `create` 只有取得资源所有权的一方才执行，不得提前打开连接再传入工厂。
    pub fn acquire<F, H>(
        resource: ResourceKey,
        configuration: LeaseConfiguration,
        protocol: &str,
        create: F,
    ) -> Result<Self, LeaseError>
    where
        F: FnOnce() -> Result<H, LeaseError>,
        H: ResourceHandler,
    {
        if protocol.is_empty() || protocol.len() > crate::MAX_RESOURCE_KEY_BYTES {
            return Err(LeaseError::LocalIpc("通信事务协议标识为空或过长".into()));
        }
        let lease = LocalLease::acquire(resource.clone(), configuration.clone())?;
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let mut create = Some(create);
        loop {
            let stream = match owner::connect(&resource) {
                Ok(stream) => Some(stream),
                Err(_) => {
                    owner::try_start(&resource, &configuration, protocol, &mut create)?;
                    owner::connect(&resource).ok()
                }
            };
            if let Some(stream) = stream {
                let mut connection = Connection::new(stream)?;
                connection.timeout(deadline.saturating_duration_since(Instant::now()))?;
                connection
                    .write_all(&RESOURCE_PROTOCOL_VERSION.to_le_bytes())
                    .map_err(io_error)?;
                write_bytes(&mut connection, protocol.as_bytes())?;
                write_bytes(&mut connection, configuration.as_bytes())?;
                match read_result(&mut connection)? {
                    Ok(_) => {
                        return Ok(Self {
                            connection: Mutex::new(Some(connection)),
                            lease,
                        });
                    }
                    Err(LeaseError::Invalidated) if Instant::now() < deadline => {}
                    Err(error) => return Err(error),
                }
            }
            if Instant::now() >= deadline {
                return Err(LeaseError::LocalIpc("等待共享资源所有者超时".into()));
            }
            thread::sleep(Duration::from_millis(2));
        }
    }

    /// 执行完整事务。超时包括代理排队和收发；租约校验另有自己的请求截止时间。
    ///
    /// 已发送请求超时后旧代理失效；未发送前的参数错误或本地排队超时不损坏会话。
    /// 已发送的事务可能仍在执行，禁止以超时为由重放有副作用的请求。
    /// 设备操作仍须有自身超时；代理不会强行中断资源所有者中的设备 I/O。
    pub fn transact(&self, request: &[u8], timeout: Duration) -> Result<Vec<u8>, LeaseError> {
        self.transact_with_timeout(request, Some(timeout))
    }

    /// 执行完整事务；None 等待实际完成，不增加隐式业务截止时间。
    /// 有截止时间时沿用 transact 的失效、不重放和设备超时规则。
    pub fn transact_with_timeout(
        &self,
        request: &[u8],
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, LeaseError> {
        if request.len() > MAX_TRANSACTION_BYTES {
            return Err(LeaseError::ConfigurationTooLarge);
        }
        let deadline = timeout
            .map(|timeout| {
                Instant::now()
                    .checked_add(timeout)
                    .filter(|_| !timeout.is_zero())
                    .ok_or_else(|| LeaseError::LocalIpc("资源请求超时必须为有限正数".into()))
            })
            .transpose()?;
        let mut state = loop {
            match self.connection.try_lock() {
                Ok(state) => break state,
                Err(std::sync::TryLockError::Poisoned(_)) => return Err(LeaseError::Invalidated),
                Err(std::sync::TryLockError::WouldBlock)
                    if deadline.is_none_or(|deadline| Instant::now() < deadline) =>
                {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(_) => return Err(LeaseError::LocalIpc("等待资源代理会话超时".into())),
            }
        };
        if state.is_none() {
            return Err(LeaseError::Invalidated);
        }
        if let Err(error) = self.lease.validate() {
            *state = None;
            return Err(error);
        }
        let connection = state.as_mut().ok_or(LeaseError::Invalidated)?;
        match deadline {
            Some(deadline) => {
                connection.timeout(deadline.saturating_duration_since(Instant::now()))?
            }
            None => connection.wait_without_deadline(),
        }
        let result = (|| {
            write_u8(connection, CALL)?;
            write_bytes(connection, request)?;
            read_result(connection)
        })();
        match result {
            Ok(Err(LeaseError::Invalidated)) => {
                *state = None;
                Err(LeaseError::Invalidated)
            }
            Ok(result) => result,
            Err(error) => {
                *state = None;
                Err(error)
            }
        }
    }

    /// 释放本调用者。最后一个代理释放时，所有者先析构物理连接，再释放其配置锁。
    pub fn close(&mut self) -> Result<(), LeaseError> {
        self.close_with(&[], Some(CLOSE_TIMEOUT)).map(|_| ())
    }

    /// 释放当前引用，将有界关闭请求传给最后一个引用触发的 ResourceHandler::close。
    /// 非最后引用返回空字节。None 等待实际关闭；请求过大或零超时不会释放引用。
    /// IPC 失败仍释放本地会话与配置租约，所有者负责在途事务和断连清理。
    pub fn close_with(
        &mut self,
        request: &[u8],
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, LeaseError> {
        if request.len() > MAX_TRANSACTION_BYTES {
            return Err(LeaseError::ConfigurationTooLarge);
        }
        if timeout.is_some_and(|timeout| {
            timeout.is_zero() || Instant::now().checked_add(timeout).is_none()
        }) {
            return Err(LeaseError::LocalIpc("资源请求超时必须为有限正数".into()));
        }
        let connection = self
            .connection
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let closed = match connection {
            Some(mut connection) => (|| {
                match timeout {
                    Some(timeout) => connection.timeout(timeout)?,
                    None => connection.wait_without_deadline(),
                }
                write_u8(&mut connection, CLOSE)?;
                write_bytes(&mut connection, request)?;
                read_result(&mut connection)?
            })(),
            None => Ok(Vec::new()),
        };
        let released = self.lease.release();
        match (closed, released) {
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(response), Ok(())) => Ok(response),
        }
    }
}

impl Drop for SharedResource {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
