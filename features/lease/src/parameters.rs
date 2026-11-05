//! 初始化参数的资源身份与配置字节编码。
use crate::{LeaseConfiguration, LeaseError, MAX_LEASE_CONFIGURATION_BYTES, ResourceKey};

/// 一个规范化初始化参数；lease 声明以 Rust 参数名引用这些值。
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

/// 由 lease 声明生成、交给通信链路实现的通用租约请求。
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
        Self::from_parameter_keys(parameters, &[resource_field], configuration_fields)
    }

    /// 以多个初始化 Config 的规范值组成资源键，配置字段独立指定；允许空配置。
    /// 单字段保留旧文本，组合字段采用 ResourceKey::from_parts 的稳定编码。
    /// 参数须已按原类型归一化；字段名和顺序决定配置字节，不访问设备或获取锁。
    ///
    /// # Errors
    /// 未找到字段返回 UnknownParameter；身份及配置的空值、保留前缀和大小约束
    /// 分别由 ResourceKey::from_parts、LeaseConfiguration::new 校验。
    pub fn from_parameter_keys(
        parameters: Vec<LeaseParameter>,
        resource_fields: &[&str],
        configuration_fields: &[&str],
    ) -> Result<Self, LeaseError> {
        let value = |field: &str| {
            parameters
                .iter()
                .find(|parameter| parameter.name == field)
                .map(|parameter| parameter.value.as_str())
                .ok_or_else(|| LeaseError::UnknownParameter(field.to_owned()))
        };
        let parts = resource_fields
            .iter()
            .map(|field| value(field))
            .collect::<Result<Vec<_>, _>>()?;
        let resource = ResourceKey::from_parts(&parts)?;
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
