//! 资源身份和不透明兼容性配置值对象。
use crate::{LeaseError, MAX_LEASE_CONFIGURATION_BYTES, MAX_RESOURCE_KEY_BYTES};
const COMPOSITE_RESOURCE_PREFIX: &str = "miresidian:tuple:v1:";

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
    /// 从有序规范值构造资源身份；单值保留原文本，多个值采用带长度边界的 v1 编码。
    /// 所有参与者须使用相同的字段顺序与类型归一化，不包含显示标签或 Rust 参数名。
    ///
    /// # Errors
    /// 空列表或空白分量返回 EmptyResource，超限返回 ResourceTooLong。
    /// 单值占用组合编码保留前缀时返回 ReservedResourcePrefix，防止单值冒充组合身份。
    pub fn from_parts(parts: &[&str]) -> Result<Self, LeaseError> {
        if parts.is_empty() || parts.iter().any(|part| part.trim().is_empty()) {
            return Err(LeaseError::EmptyResource);
        }
        if parts.len() == 1 {
            if parts[0].starts_with(COMPOSITE_RESOURCE_PREFIX) {
                return Err(LeaseError::ReservedResourcePrefix);
            }
            return Self::new(parts[0]);
        }
        let mut key = format!("{COMPOSITE_RESOURCE_PREFIX}{}:", parts.len());
        for part in parts {
            if part.len() > MAX_RESOURCE_KEY_BYTES {
                return Err(LeaseError::ResourceTooLong);
            }
            use std::fmt::Write;
            let _ = write!(key, "{}:{part}", part.len());
            if key.len() > MAX_RESOURCE_KEY_BYTES {
                return Err(LeaseError::ResourceTooLong);
            }
        }
        Self::new(key)
    }

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
