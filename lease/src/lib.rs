//! 设备类型无关的资源租约协议。
//!
//! 协议将“资源身份”“配置一致性”和“租约所有权”分离；框架宏负责规范化节点初始化参数。
//! 本文件维护协议值对象和内存状态机；跨 DLL 的本地 IPC 实现在 `local_ipc` 子模块。

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

/// 此租约协议的当前版本。
pub const LEASE_PROTOCOL_VERSION: u16 = 2;
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
    /// 服务连接、服务代次或会话校验已失效；必须关闭资源后重新获取。
    Invalidated,
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
            Self::Invalidated => "租约会话已失效；请关闭资源后重新获取租约",
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

    /// 释放租约；`was_last` 仅表示本登记表中最后一个令牌，不代表跨代物理所有权。
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
pub mod local_ipc;

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
    first.validate().expect("首个会话仍有效");
    shared.validate().expect("共享会话仍有效");
    assert_eq!(first.generation(), shared.generation());
    first.release().expect("首个客户端可释放租约");
    assert!(first.validate().is_err(), "已释放的会话不能继续校验");
    first.release().expect("重复释放幂等");
    shared.release().expect("共享客户端可释放租约");
}
