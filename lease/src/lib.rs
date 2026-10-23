//! 设备类型无关的资源租约协议。
//!
//! 协议将“资源身份”“配置一致性”和“租约所有权”分离；框架宏负责规范化节点初始化参数。

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

#[cfg(feature = "local-ipc")]
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

/// 此租约协议的当前版本。
pub const LEASE_PROTOCOL_VERSION: u16 = 1;
/// 单个逻辑资源键允许的最大 UTF-8 字节数。
pub const MAX_RESOURCE_KEY_BYTES: usize = 512;
/// 设备专属规范化配置允许的最大字节数。
pub const MAX_LEASE_CONFIGURATION_BYTES: usize = 64 * 1024;

/// 一个设备无关、稳定的逻辑资源标识，例如 `serial:COM3` 或 `can:adapter-1`。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// 创建已验证的资源键。
    pub fn new(value: impl Into<String>) -> Result<Self, LeaseError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(LeaseError::EmptyResource);
        }
        if value.len() > MAX_RESOURCE_KEY_BYTES {
            return Err(LeaseError::ResourceTooLong);
        }
        Ok(Self(value))
    }
}

impl ResourceKey {
    /// 返回资源键的规范文本。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 某项资源所需的设备专属配置的规范化不透明字节。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseConfiguration(Vec<u8>);

impl LeaseConfiguration {
    /// 创建配置。调用方应使用稳定的规范化格式或其摘要。
    pub fn new(bytes: Vec<u8>) -> Result<Self, LeaseError> {
        if bytes.len() > MAX_LEASE_CONFIGURATION_BYTES {
            return Err(LeaseError::ConfigurationTooLarge);
        }
        Ok(Self(bytes))
    }

    /// 返回原始规范化配置字节。
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// 一个规范化初始化参数；结构体级 lease 标记以 Rust 参数名引用这些值。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseParameter {
    name: String,
    value: String,
}

impl LeaseParameter {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

/// 由结构体级 lease 声明生成、交给通信链路实现的通用租约请求。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseRequest {
    resource: ResourceKey,
    configuration: LeaseConfiguration,
    parameters: Vec<LeaseParameter>,
}

impl LeaseRequest {
    pub fn from_parameters(
        parameters: Vec<LeaseParameter>,
        resource_field: &str,
        configuration_fields: &[&str],
    ) -> Result<Self, LeaseError> {
        let value = |field: &str| {
            parameters
                .iter()
                .find(|parameter| parameter.name == field)
                .map(|parameter| parameter.value.as_str())
                .ok_or_else(|| LeaseError::UnknownParameter(field.to_owned()))
        };
        let resource = ResourceKey::new(value(resource_field)?.to_owned())?;
        let mut encoded = Vec::new();
        for field in configuration_fields {
            let parameter = value(field)?;
            push_length_prefixed(&mut encoded, field.as_bytes())?;
            push_length_prefixed(&mut encoded, parameter.as_bytes())?;
        }
        Ok(Self {
            resource,
            configuration: LeaseConfiguration::new(encoded)?,
            parameters,
        })
    }

    pub fn resource(&self) -> &ResourceKey {
        &self.resource
    }

    pub fn configuration(&self) -> &LeaseConfiguration {
        &self.configuration
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name == name)
            .map(LeaseParameter::value)
    }
}

fn push_length_prefixed(target: &mut Vec<u8>, value: &[u8]) -> Result<(), LeaseError> {
    let length = u32::try_from(value.len()).map_err(|_| LeaseError::ConfigurationTooLarge)?;
    target.extend_from_slice(&length.to_le_bytes());
    target.extend_from_slice(value);
    if target.len() > MAX_LEASE_CONFIGURATION_BYTES {
        return Err(LeaseError::ConfigurationTooLarge);
    }
    Ok(())
}

/// 由操作系统随机源生成的、不透明租约标识。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LeaseId([u8; 32]);

impl LeaseId {
    /// 生成新的 256 位租约标识；随机源不可用时不会降级为可预测标识。
    pub fn random() -> Result<Self, LeaseError> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| LeaseError::Randomness)?;
        Ok(Self(bytes))
    }

    /// 返回仅用于二进制协议传输的租约字节。
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// 通用租约协议返回的可诊断错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LeaseError {
    EmptyResource,
    ResourceTooLong,
    ConfigurationTooLarge,
    Randomness,
    IncompatibleConfiguration,
    UnknownLease,
    UnknownParameter(String),
    LocalIpc(String),
}

impl fmt::Display for LeaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyResource => "资源键不能为空",
            Self::ResourceTooLong => "资源键过长",
            Self::ConfigurationTooLarge => "资源配置过长",
            Self::Randomness => "无法从操作系统随机源生成租约标识",
            Self::IncompatibleConfiguration => "资源已由不兼容配置持有",
            Self::UnknownLease => "租约不存在或已释放",
            Self::UnknownParameter(name) => {
                return write!(formatter, "租约声明引用了不存在的初始化参数 {name}");
            }
            Self::LocalIpc(message) => return formatter.write_str(message),
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LeaseError {}

/// 当前获取操作在资源生命周期中的位置。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeaseState {
    First,
    Shared,
}

/// 成功获取租约后的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseGrant {
    pub lease: LeaseId,
    pub state: LeaseState,
}

/// 成功释放租约后的资源生命周期结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseRelease {
    pub resource: ResourceKey,
    pub was_last: bool,
}

/// 持有资源配置与租约归属的通用状态机；外层可用 `Mutex` 共享给本地 IPC 服务。
#[derive(Default)]
pub struct LeaseRegistry {
    resources: HashMap<ResourceKey, ResourceEntry>,
    leases: HashMap<LeaseId, ResourceKey>,
}

struct ResourceEntry {
    configuration: LeaseConfiguration,
    leases: HashSet<LeaseId>,
}

impl LeaseRegistry {
    /// 获取资源租约；相同资源只能复用完全相同的规范化配置。
    pub fn acquire(
        &mut self,
        resource: ResourceKey,
        configuration: LeaseConfiguration,
    ) -> Result<LeaseGrant, LeaseError> {
        let lease = LeaseId::random()?;
        let state = if let Some(entry) = self.resources.get_mut(&resource) {
            if entry.configuration != configuration {
                return Err(LeaseError::IncompatibleConfiguration);
            }
            entry.leases.insert(lease);
            LeaseState::Shared
        } else {
            let mut leases = HashSet::new();
            leases.insert(lease);
            self.resources.insert(
                resource.clone(),
                ResourceEntry {
                    configuration,
                    leases,
                },
            );
            LeaseState::First
        };
        self.leases.insert(lease, resource);
        Ok(LeaseGrant { lease, state })
    }

    /// 验证租约是否仍属于给定资源。
    pub fn validate(&self, resource: &ResourceKey, lease: LeaseId) -> bool {
        self.leases
            .get(&lease)
            .is_some_and(|actual| actual == resource)
    }

    /// 释放租约；`was_last` 为真时，节点应关闭对应物理资源。
    pub fn release(&mut self, lease: LeaseId) -> Result<LeaseRelease, LeaseError> {
        let resource = self.leases.remove(&lease).ok_or(LeaseError::UnknownLease)?;
        let Some(entry) = self.resources.get_mut(&resource) else {
            return Err(LeaseError::UnknownLease);
        };
        entry.leases.remove(&lease);
        let was_last = entry.leases.is_empty();
        if was_last {
            self.resources.remove(&resource);
        }
        Ok(LeaseRelease { resource, was_last })
    }

    /// 返回所有资源上仍存活的租约数。
    pub fn active_lease_count(&self) -> usize {
        self.leases.len()
    }

    /// 返回仍由至少一个租约占用的资源数。
    pub fn active_resource_count(&self) -> usize {
        self.resources.len()
    }
}

/// 平台映射为 Windows Named Pipe 或 Unix domain socket 的同步本地 IPC 原语。
#[cfg(feature = "local-ipc")]
pub mod local_ipc {
    pub use interprocess::local_socket::{
        ConnectOptions, GenericNamespaced, Listener, ListenerOptions, Stream, ToNsName, prelude::*,
    };
    use std::io;

    use super::{
        Arc, Duration, LEASE_PROTOCOL_VERSION, LeaseConfiguration, LeaseError, LeaseGrant, LeaseId,
        LeaseRegistry, LeaseState, MAX_LEASE_CONFIGURATION_BYTES, MAX_RESOURCE_KEY_BYTES, Mutex,
        Read, ResourceKey, Write, thread,
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
                    .map_err(|error| {
                        LeaseError::LocalIpc(format!("无法启动本地租约服务：{error}"))
                    })?;
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
}

#[cfg(test)]
mod tests {
    use super::{LeaseConfiguration, LeaseError, LeaseId, LeaseRegistry, LeaseState, ResourceKey};

    #[test]
    fn compatible_leases_share_one_resource_until_the_last_release() {
        let resource = ResourceKey::new("serial:COM7").expect("资源键有效");
        let configuration = LeaseConfiguration::new(vec![1, 2, 3]).expect("配置有效");
        let mut registry = LeaseRegistry::default();
        let first = registry
            .acquire(resource.clone(), configuration.clone())
            .expect("首个租约应成功");
        assert_eq!(first.state, LeaseState::First);
        assert!(registry.validate(&resource, first.lease));
        let shared = registry
            .acquire(resource.clone(), configuration)
            .expect("兼容配置可共享");
        assert_eq!(shared.state, LeaseState::Shared);
        assert_eq!(registry.active_lease_count(), 2);
        assert_eq!(
            registry.acquire(
                resource.clone(),
                LeaseConfiguration::new(vec![9]).expect("配置有效")
            ),
            Err(LeaseError::IncompatibleConfiguration)
        );
        let released = registry.release(first.lease).expect("首个租约可释放");
        assert!(!released.was_last);
        assert!(registry.validate(&resource, shared.lease));
        assert!(
            registry
                .release(shared.lease)
                .expect("末个租约可释放")
                .was_last
        );
        assert_eq!(registry.active_resource_count(), 0);
        assert_eq!(
            registry.release(shared.lease),
            Err(LeaseError::UnknownLease)
        );
    }

    #[test]
    fn generated_lease_ids_are_not_reused() {
        assert_ne!(LeaseId::random().unwrap(), LeaseId::random().unwrap());
    }
}

#[cfg(feature = "local-ipc")]
#[test]
fn 本地_ipc_在独立客户端连接间共享租约状态() {
    use crate::local_ipc::LocalLease;

    let resource =
        ResourceKey::new(format!("test:ipc:{}", std::process::id())).expect("测试资源键有效");
    let configuration = LeaseConfiguration::new(vec![7, 8, 9]).expect("测试配置有效");
    let mut first =
        LocalLease::acquire(resource.clone(), configuration.clone()).expect("首个客户端可取得租约");
    let mut shared = LocalLease::acquire(resource.clone(), configuration.clone())
        .expect("第二个独立客户端可取得共享租约");
    assert_eq!(first.state(), LeaseState::First);
    assert_eq!(shared.state(), LeaseState::Shared);
    assert!(
        LocalLease::acquire(
            resource,
            LeaseConfiguration::new(vec![1]).expect("测试配置有效"),
        )
        .is_err(),
        "不兼容配置必须被另一客户端持有的租约拒绝"
    );
    first.release().expect("首个客户端可释放租约");
    shared.release().expect("共享客户端可释放租约");
}
