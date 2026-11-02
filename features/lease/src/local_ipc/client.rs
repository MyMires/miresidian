//! 有界 IPC 客户端；失效会话不会重连或自动重新申请租约。
use std::{
    io::{self, Read, Write},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use interprocess::local_socket::{Stream, prelude::*};

use super::{protocol::*, resource::ResourceGuard, service::connect_or_start};
use crate::{
    LEASE_PROTOCOL_VERSION, LeaseConfiguration, LeaseError, LeaseId, LeaseState, ResourceKey,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const RETRY_INTERVAL: Duration = Duration::from_millis(2);

/// 与服务代次绑定的租约；持有期间始终保护资源配置的一致性。
///
/// `validate` 失败后必须先关闭设备再释放租约。租约仅协调配置，不共享设备句柄，
/// 不提供跨进程设备事务锁；业务节点仍负责物理连接与请求响应的排他性。
pub struct LocalLease {
    client: Mutex<Option<Connection>>,
    resource_guard: Option<ResourceGuard>,
    id: LeaseId,
    generation: LeaseId,
    state: LeaseState,
    released: bool,
}

impl LocalLease {
    /// 获取兼容租约。跨进程配置锁先于 IPC 获取，服务重启也不能绕过存活持有者。
    pub fn acquire(
        resource: ResourceKey,
        configuration: LeaseConfiguration,
    ) -> Result<Self, LeaseError> {
        let resource_guard = ResourceGuard::acquire(&resource, &configuration)?;
        let mut client = Connection::new(connect_or_start()?)?;
        write_u8(&mut client, ACQUIRE)?;
        client
            .write_all(&LEASE_PROTOCOL_VERSION.to_le_bytes())
            .map_err(io_error)?;
        write_bytes(&mut client, resource.as_str().as_bytes())?;
        write_bytes(&mut client, configuration.as_bytes())?;
        let (grant, generation) = read_grant(&mut client)?;
        Ok(Self {
            client: Mutex::new(Some(client)),
            resource_guard: Some(resource_guard),
            id: grant.lease,
            generation,
            state: grant.state,
            released: false,
        })
    }

    /// 返回不透明租约 ID。
    pub fn id(&self) -> LeaseId {
        self.id
    }

    /// 返回服务端登记本租约时的状态；`First` 不表示独占物理设备。
    pub fn state(&self) -> LeaseState {
        self.state
    }

    /// 返回授予此租约的服务代次；服务重启后生成新值。
    pub fn generation(&self) -> LeaseId {
        self.generation
    }

    /// 在原连接上校验租约与代次。失联、超时或校验失败会永久失效此会话。
    ///
    /// 返回错误后禁止开始新的设备操作；不会自动重连。已开始的同步设备操作仍由节点的
    /// 超时和取消机制约束。失效租约保留配置锁，直到设备关闭后调用 `release` 或析构。
    pub fn validate(&self) -> Result<(), LeaseError> {
        let mut client = self.client.lock().map_err(|_| LeaseError::Invalidated)?;
        let result = client
            .as_mut()
            .ok_or(LeaseError::Invalidated)
            .and_then(|client| {
                client.reset_deadline();
                write_u8(client, VALIDATE)?;
                write_session(client, self.id, self.generation)?;
                read_empty_response(client)
            });
        if result.is_err() {
            *client = None;
        }
        result
    }

    /// 设备关闭后释放租约。IPC 失败时仍关闭连接、释放配置锁，并返回诊断错误。
    pub fn release(&mut self) -> Result<(), LeaseError> {
        if self.released {
            return Ok(());
        }
        self.released = true;
        let client = self
            .client
            .get_mut()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let result = match client {
            Some(mut client) => {
                client.reset_deadline();
                (|| {
                    write_u8(&mut client, RELEASE)?;
                    write_session(&mut client, self.id, self.generation)?;
                    read_empty_response(&mut client)
                })()
            }
            None => Err(LeaseError::Invalidated),
        };
        self.resource_guard.take();
        result
    }
}

impl Drop for LocalLease {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

// Windows 同步命名管道没有接收超时，使用非阻塞读写与统一请求截止时间。
struct Connection {
    stream: Stream,
    deadline: Instant,
}

impl Connection {
    fn new(stream: Stream) -> Result<Self, LeaseError> {
        stream.set_nonblocking(true).map_err(io_error)?;
        Ok(Self {
            stream,
            deadline: Instant::now() + REQUEST_TIMEOUT,
        })
    }

    fn reset_deadline(&mut self) {
        self.deadline = Instant::now() + REQUEST_TIMEOUT;
    }

    fn transfer(
        &mut self,
        mut action: impl FnMut(&mut Stream) -> io::Result<usize>,
    ) -> io::Result<usize> {
        loop {
            if Instant::now() >= self.deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "本地租约请求超时"));
            }
            match action(&mut self.stream) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(RETRY_INTERVAL)
                }
                result => return result,
            }
        }
    }
}

impl Read for Connection {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        self.transfer(|stream| {
            #[cfg(windows)]
            {
                read_windows_pipe(stream, bytes)
            }
            #[cfg(not(windows))]
            {
                stream.read(bytes)
            }
        })
    }
}

impl Write for Connection {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.transfer(|stream| match stream.write(bytes) {
            // 非阻塞 Windows 管道缓冲区满时可能成功写入 0 字节；等待后重试。
            Ok(0) if !bytes.is_empty() => Err(io::ErrorKind::WouldBlock.into()),
            result => result,
        })
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(windows)]
fn read_windows_pipe(stream: &mut Stream, bytes: &mut [u8]) -> io::Result<usize> {
    use std::{
        ffi::c_void,
        os::windows::io::{AsHandle, AsRawHandle},
        ptr,
    };

    unsafe extern "system" {
        fn PeekNamedPipe(
            pipe: *mut c_void,
            buffer: *mut c_void,
            buffer_size: u32,
            bytes_read: *mut u32,
            bytes_available: *mut u32,
            bytes_left: *mut u32,
        ) -> i32;
    }
    let Stream::NamedPipe(pipe) = stream;
    let mut available = 0;
    // SAFETY: pipe 持有存活的管道句柄，available 是有效输出地址；其他可选输出为空。
    // Connection 由租约锁保护，是此句柄的唯一读取者，探测后不会被另一读取者抢走数据。
    let success = unsafe {
        PeekNamedPipe(
            pipe.as_handle().as_raw_handle(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &mut available,
            ptr::null_mut(),
        )
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    if available == 0 {
        // interprocess 会把 ERROR_NO_DATA 映射成 EOF；有数据后才调用其读取入口。
        return Err(io::ErrorKind::WouldBlock.into());
    }
    let length = bytes.len().min(available as usize);
    pipe.read(&mut bytes[..length])
}

#[cfg(test)]
mod tests {
    use super::*;
    use interprocess::local_socket::{
        ConnectOptions, GenericNamespaced, ListenerOptions, ToNsName,
    };

    #[test]
    fn stalled_peer_obeys_request_deadline() {
        let name = format!("miresidian-timeout-{}", std::process::id());
        let listener = ListenerOptions::new()
            .name(name.as_str().to_ns_name::<GenericNamespaced>().unwrap())
            .create_sync()
            .unwrap();
        let (stop, stopped) = std::sync::mpsc::channel();
        let peer = thread::spawn(move || {
            let _stream = listener.accept().unwrap();
            let _ = stopped.recv_timeout(Duration::from_secs(5));
        });
        let stream = ConnectOptions::new()
            .name(name.as_str().to_ns_name::<GenericNamespaced>().unwrap())
            .connect_sync()
            .unwrap();
        let mut client = Connection::new(stream).unwrap();
        let started = Instant::now();
        let error = client.read_exact(&mut [0]).unwrap_err();
        let _ = stop.send(());
        peer.join().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(4));
    }
}
