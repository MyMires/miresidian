//! 跨 DLL 本地租约服务的传输、客户端与服务循环。
//!
//! 该模块依赖父模块的协议值对象和状态机，不定义任何设备专属资源语义。

pub use interprocess::local_socket::{
    ConnectOptions, GenericNamespaced, Listener, ListenerOptions, Stream, ToNsName, prelude::*,
};
use std::io;

use super::{
    Arc, Duration, LEASE_PROTOCOL_VERSION, LeaseConfiguration, LeaseError, LeaseGrant, LeaseId,
    LeaseRegistry, LeaseState, MAX_LEASE_CONFIGURATION_BYTES, MAX_RESOURCE_KEY_BYTES, Mutex, Read,
    ResourceKey, Write, thread,
};

/// 所有同一用户上下文中的 Miresidian DLL 共用的租约协调端点。
const SERVICE_NAME: &str = "miresidian-lease-v1";
const ACQUIRE: u8 = 1;
const RELEASE: u8 = 2;
const RESPONSE_OK: u8 = 0;
const RESPONSE_ERROR: u8 = 1;

/// 与框架本地租约服务保持连接的一份租约。
///
/// 服务端把连接的意外断开视为释放，因此 DLL 崩溃或卸载不会永久遗留租约。
pub struct LocalLease {
    stream: Stream,
    id: LeaseId,
    state: LeaseState,
    released: bool,
}

impl LocalLease {
    /// 取得跨 DLL 可见的资源租约。服务尚不存在时当前调用方创建它。
    pub fn acquire(
        resource: ResourceKey,
        configuration: LeaseConfiguration,
    ) -> Result<Self, LeaseError> {
        let mut stream = connect_or_start()?;
        write_u8(&mut stream, ACQUIRE)?;
        stream
            .write_all(&LEASE_PROTOCOL_VERSION.to_le_bytes())
            .map_err(io_error)?;
        write_bytes(&mut stream, resource.as_str().as_bytes())?;
        write_bytes(&mut stream, configuration.as_bytes())?;
        let grant = read_grant(&mut stream)?;
        Ok(Self {
            stream,
            id: grant.lease,
            state: grant.state,
            released: false,
        })
    }

    /// 返回服务端分配的租约标识。
    pub fn id(&self) -> LeaseId {
        self.id
    }

    /// 返回此租约是首个持有者还是共享持有者。
    pub fn state(&self) -> LeaseState {
        self.state
    }

    /// 显式释放租约并等待服务端确认。
    pub fn release(&mut self) -> Result<(), LeaseError> {
        if self.released {
            return Ok(());
        }
        write_u8(&mut self.stream, RELEASE)?;
        self.stream
            .write_all(self.id.as_bytes())
            .map_err(io_error)?;
        read_empty_response(&mut self.stream)?;
        self.released = true;
        Ok(())
    }
}

impl Drop for LocalLease {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

fn connect_or_start() -> Result<Stream, LeaseError> {
    if let Ok(stream) = connect() {
        return Ok(stream);
    }

    let name = SERVICE_NAME
        .to_ns_name::<GenericNamespaced>()
        .map_err(io_error)?;
    match ListenerOptions::new().name(name).create_sync() {
        Ok(listener) => {
            pin_service_module()?;
            thread::Builder::new()
                .name("miresidian-lease-service".to_owned())
                .spawn(move || serve(listener))
                .map_err(|error| LeaseError::LocalIpc(format!("无法启动本地租约服务：{error}")))?;
        }
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {}
        Err(error) => return Err(io_error(error)),
    }

    for _ in 0..50 {
        if let Ok(stream) = connect() {
            return Ok(stream);
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(LeaseError::LocalIpc(
        "无法连接 Miresidian 本地租约服务".to_owned(),
    ))
}

fn connect() -> Result<Stream, LeaseError> {
    let name = SERVICE_NAME
        .to_ns_name::<GenericNamespaced>()
        .map_err(io_error)?;
    ConnectOptions::new()
        .name(name)
        .connect_sync()
        .map_err(io_error)
}

/// 服务线程执行框架代码，因此 Windows 上固定首个承载 DLL，直至进程退出。
#[cfg(windows)]
fn pin_service_module() -> Result<(), LeaseError> {
    unsafe extern "system" {
        fn GetModuleHandleExW(
            flags: u32,
            module_name: *const u16,
            module: *mut *mut core::ffi::c_void,
        ) -> i32;
    }

    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
    let mut module = core::ptr::null_mut();
    // SAFETY: FROM_ADDRESS 使用当前模块内的函数地址，输出参数是有效局部指针。
    let succeeded = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            pin_service_module as *const () as *const u16,
            &mut module,
        )
    };
    if succeeded == 0 || module.is_null() {
        return Err(LeaseError::LocalIpc(
            "无法固定本地租约服务宿主 DLL".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn pin_service_module() -> Result<(), LeaseError> {
    Ok(())
}

fn serve(listener: Listener) {
    let registry = Arc::new(Mutex::new(LeaseRegistry::default()));
    for stream in listener.incoming().flatten() {
        let registry = Arc::clone(&registry);
        let _ = thread::Builder::new()
            .name("miresidian-lease-client".to_owned())
            .spawn(move || serve_client(stream, registry));
    }
}

fn serve_client(mut stream: Stream, registry: Arc<Mutex<LeaseRegistry>>) {
    let request = read_acquire(&mut stream);
    let grant = match request {
        Ok((resource, configuration)) => registry
            .lock()
            .map_err(|_| LeaseError::LocalIpc("本地租约状态锁已损坏".to_owned()))
            .and_then(|mut registry| registry.acquire(resource, configuration)),
        Err(error) => Err(error),
    };
    let Ok(grant) = grant else {
        let _ = write_error(&mut stream, grant.expect_err("分支已确认失败"));
        return;
    };
    if write_grant(&mut stream, &grant).is_err() {
        let _ = release(&registry, grant.lease);
        return;
    }

    let mut held = Some(grant.lease);
    loop {
        match read_u8(&mut stream) {
            Ok(RELEASE) => {
                let mut bytes = [0; 32];
                if stream.read_exact(&mut bytes).is_err() || bytes != *grant.lease.as_bytes() {
                    let _ = write_error(&mut stream, LeaseError::UnknownLease);
                    continue;
                }
                let response = release(&registry, grant.lease);
                let _ = match response {
                    Ok(()) => write_empty_response(&mut stream),
                    Err(error) => write_error(&mut stream, error),
                };
                held = None;
                break;
            }
            Ok(_) => {
                let _ = write_error(
                    &mut stream,
                    LeaseError::LocalIpc("本地租约服务收到不支持的请求".to_owned()),
                );
            }
            Err(_) => break,
        }
    }
    if let Some(lease) = held {
        let _ = release(&registry, lease);
    }
}

fn release(registry: &Arc<Mutex<LeaseRegistry>>, lease: LeaseId) -> Result<(), LeaseError> {
    registry
        .lock()
        .map_err(|_| LeaseError::LocalIpc("本地租约状态锁已损坏".to_owned()))?
        .release(lease)
        .map(|_| ())
}

fn read_acquire(stream: &mut Stream) -> Result<(ResourceKey, LeaseConfiguration), LeaseError> {
    if read_u8(stream)? != ACQUIRE {
        return Err(LeaseError::LocalIpc("本地租约请求类型无效".to_owned()));
    }
    let mut version = [0; 2];
    stream.read_exact(&mut version).map_err(io_error)?;
    if u16::from_le_bytes(version) != LEASE_PROTOCOL_VERSION {
        return Err(LeaseError::LocalIpc("本地租约协议版本不兼容".to_owned()));
    }
    let resource = String::from_utf8(read_bytes(stream, MAX_RESOURCE_KEY_BYTES)?)
        .map_err(|_| LeaseError::LocalIpc("资源键不是有效 UTF-8".to_owned()))?;
    Ok((
        ResourceKey::new(resource)?,
        LeaseConfiguration::new(read_bytes(stream, MAX_LEASE_CONFIGURATION_BYTES)?)?,
    ))
}

fn read_grant(stream: &mut Stream) -> Result<LeaseGrant, LeaseError> {
    read_status(stream)?;
    let mut id = [0; 32];
    stream.read_exact(&mut id).map_err(io_error)?;
    let state = match read_u8(stream)? {
        0 => LeaseState::First,
        1 => LeaseState::Shared,
        _ => return Err(LeaseError::LocalIpc("本地租约状态无效".to_owned())),
    };
    Ok(LeaseGrant {
        lease: LeaseId(id),
        state,
    })
}

fn write_grant(stream: &mut Stream, grant: &LeaseGrant) -> Result<(), LeaseError> {
    write_u8(stream, RESPONSE_OK)?;
    stream.write_all(grant.lease.as_bytes()).map_err(io_error)?;
    write_u8(
        stream,
        match grant.state {
            LeaseState::First => 0,
            LeaseState::Shared => 1,
        },
    )
}

fn read_empty_response(stream: &mut Stream) -> Result<(), LeaseError> {
    read_status(stream)
}

fn write_empty_response(stream: &mut Stream) -> Result<(), LeaseError> {
    write_u8(stream, RESPONSE_OK)
}

fn read_status(stream: &mut Stream) -> Result<(), LeaseError> {
    match read_u8(stream)? {
        RESPONSE_OK => Ok(()),
        RESPONSE_ERROR => Err(LeaseError::LocalIpc(
            String::from_utf8_lossy(&read_bytes(stream, MAX_LEASE_CONFIGURATION_BYTES)?)
                .into_owned(),
        )),
        _ => Err(LeaseError::LocalIpc("本地租约响应无效".to_owned())),
    }
}

fn write_error(stream: &mut Stream, error: LeaseError) -> Result<(), LeaseError> {
    write_u8(stream, RESPONSE_ERROR)?;
    write_bytes(stream, error.to_string().as_bytes())
}

fn read_u8(stream: &mut Stream) -> Result<u8, LeaseError> {
    let mut value = [0; 1];
    stream.read_exact(&mut value).map_err(io_error)?;
    Ok(value[0])
}

fn write_u8(stream: &mut Stream, value: u8) -> Result<(), LeaseError> {
    stream.write_all(&[value]).map_err(io_error)
}

fn read_bytes(stream: &mut Stream, maximum: usize) -> Result<Vec<u8>, LeaseError> {
    let mut length = [0; 4];
    stream.read_exact(&mut length).map_err(io_error)?;
    let length = usize::try_from(u32::from_le_bytes(length))
        .map_err(|_| LeaseError::ConfigurationTooLarge)?;
    if length > maximum {
        return Err(LeaseError::ConfigurationTooLarge);
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(io_error)?;
    Ok(bytes)
}

fn write_bytes(stream: &mut Stream, bytes: &[u8]) -> Result<(), LeaseError> {
    let length = u32::try_from(bytes.len()).map_err(|_| LeaseError::ConfigurationTooLarge)?;
    stream.write_all(&length.to_le_bytes()).map_err(io_error)?;
    stream.write_all(bytes).map_err(io_error)
}

fn io_error(error: io::Error) -> LeaseError {
    LeaseError::LocalIpc(format!("本地租约 IPC 失败：{error}"))
}
