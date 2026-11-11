//! 节点与操作的编译期元数据。

use super::instances::{ConfigLayout, HANDLE_DEFINITION, InstancePolicy};

pub use miresidian_instances::ExecutionMode;

/// 框架自动追加到 `InitDevice` 的多通道调度配置描述。
pub const SHARED_EXECUTION_MODE_DEFINITION: &str =
    "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播";

/// 历史平台选择参数名称，仅用于拒绝旧 Config 与诊断；不再发布为参数。
pub const PLATFORM_ADAPTER_NAME: &str = "平台适配";

/// 节点租约的激活策略。`Selectable` 由平台在初始化时选择访问路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseActivation {
    Required,
    Disabled,
    Selectable,
}

impl LeaseActivation {
    pub const fn has_platform_selector(self) -> bool {
        matches!(self, Self::Selectable)
    }
}

/// 单次初始化实际采用的资源访问路径。
/// Lease 自动共用唯一节点资源对象和业务状态；Direct 本地构造。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseAccess {
    Lease,
    Direct,
}

impl LeaseActivation {
    pub fn resolve_access(self, value: Option<&str>) -> LeaseAccess {
        match self {
            Self::Required => LeaseAccess::Lease,
            Self::Disabled => LeaseAccess::Direct,
            Self::Selectable => match value
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str()
            {
                "direct" | "直连" => LeaseAccess::Direct,
                _ => LeaseAccess::Lease,
            },
        }
    }
}
/// `Selectable` 租约节点自动追加的资源访问方式配置。
pub const RESOURCE_ACCESS_DEFINITION: &str = "资源访问方式:Lease-租约/Direct-直连";

/// 配置值、输入值或输出值的元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterDescriptor {
    /// 供错误诊断使用的参数名称。
    pub name: &'static str,
    /// 原始参数声明；显式非空描述保留，自动候选由 definition_for 按平台渲染。
    pub definition: &'static str,
    /// 明确指定的框架选择器或手写契约默认值；业务参数宏不从描述推导此值。
    pub default_value: Option<&'static str>,
    /// 原 Rust 输入类型提供的结构化候选，格式与输入表示由平台适配器决定。
    pub enum_options: &'static [crate::EnumOption],
}

impl ParameterDescriptor {
    /// 原样返回声明，不选择平台或生成候选。平台查询使用 definition_for。
    pub fn definition(&self) -> String {
        self.definition.to_owned()
    }

    /// 是否应使用类型候选；已有非空默认/候选描述优先保留。
    pub fn has_auto_options(&self) -> bool {
        !self.enum_options.is_empty()
            && self
                .definition
                .split_once(':')
                .is_none_or(|(_, rest)| rest.trim().is_empty())
    }

    /// 使用调用方明确指定的平台生成参数描述。
    pub fn definition_for(
        &self,
        adapter: &dyn crate::PlatformAdapter,
    ) -> crate::MiresidianResult<String> {
        adapter.format_parameter(self)
    }
}

/// 单个公开节点操作的元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationDescriptor {
    /// 面向平台的操作名称。
    pub name: &'static str,
    /// 该操作接受的有序输入。
    pub inputs: &'static [ParameterDescriptor],
    /// 该操作返回的有序输出。
    pub outputs: &'static [ParameterDescriptor],
    /// 多通道调用时用于判定通道参数串合法性的 Input 来源参数；不含 `config` 标注参数。
    pub channel_inputs: &'static [ParameterDescriptor],
}

impl OperationDescriptor {
    /// 按明确的平台格式发布操作参数；不缓存其他平台的结果。
    pub fn signature_for(
        &self,
        adapter: &dyn crate::PlatformAdapter,
    ) -> crate::MiresidianResult<String> {
        Ok(format!(
            "{}({})({})",
            self.name,
            definitions_for(self.inputs, adapter)?,
            definitions_for(self.outputs, adapter)?
        ))
    }

    /// 生成原始声明签名，不展开自动候选；面向宿主发布时使用 signature_for。
    pub fn signature(&self) -> String {
        format!(
            "{}({})({})",
            self.name,
            definitions(self.inputs),
            definitions(self.outputs)
        )
    }
}

/// 业务节点的编译期元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeDescriptor {
    /// 用于诊断及后续发现机制的稳定节点名称。
    pub name: &'static str,
    /// 初始化业务 Config；组合身份由 Node::instance_config_indices 选择，框架保留字段不在此切片中。
    pub init_config: &'static [ParameterDescriptor],
    /// `InitDevice` 的有序输入值；它们不属于通道身份。
    pub init_inputs: &'static [ParameterDescriptor],
    /// `InitDevice` 的有序输出。
    pub init_outputs: &'static [ParameterDescriptor],
    /// `GetPortNO` 的有序输入；为空表示该驱动不需要探测输入。
    pub port_probe_inputs: &'static [ParameterDescriptor],
    /// 节点自定义的平台操作。
    pub operations: &'static [OperationDescriptor],
    /// 租约启用策略；由 `#[node(..., lease(...))]` 声明。
    pub lease_activation: LeaseActivation,
    /// 实例数量、关联键及执行策略；未标注节点使用 `InstancePolicy::DEFAULT`。
    pub instances: InstancePolicy,
}

impl NodeDescriptor {
    /// 由调用平台生成初始化 Config 描述，不包含运行时平台选择器。
    pub fn init_config_definitions_for(
        &self,
        adapter: &dyn crate::PlatformAdapter,
    ) -> crate::MiresidianResult<Vec<String>> {
        let mut values = self.config_definitions();
        for (value, parameter) in values.iter_mut().zip(self.init_config) {
            *value = parameter.definition_for(adapter)?;
        }
        Ok(values)
    }

    /// 由调用平台生成所有查询、生命周期及业务操作的参数描述。
    pub fn function_signatures_for(
        &self,
        adapter: &dyn crate::PlatformAdapter,
    ) -> crate::MiresidianResult<Vec<String>> {
        let config = self.init_config_definitions_for(adapter)?;
        let mut init = config.clone();
        init.extend(
            self.init_inputs
                .iter()
                .map(|p| p.definition_for(adapter))
                .collect::<crate::MiresidianResult<Vec<_>>>()?,
        );
        let mut values = vec![
            "GetFunctions()(函数列表)".to_owned(),
            "GetFrameworkVersion()(框架版本)".to_owned(),
            "GetProjectVersion()(项目版本)".to_owned(),
            format!("GetInitConfig()({})", config.join(",")),
            format!(
                "GetPortNO({})(搜索结果,端口号)",
                definitions_for(self.port_probe_inputs, adapter)?
            ),
            format!(
                "InitDevice({})({})",
                init.join(","),
                definitions_for(self.init_outputs, adapter)?
            ),
            "CloseDevice()()".to_owned(),
            "CloseAllDevices()()".to_owned(),
        ];
        values.extend(
            self.operations
                .iter()
                .map(|operation| operation.signature_for(adapter))
                .collect::<crate::MiresidianResult<Vec<_>>>()?,
        );
        Ok(values)
    }

    /// 返回全部操作的原始声明签名，不展开自动候选；宿主查询使用 function_signatures_for。
    pub fn function_signatures(&self) -> Vec<String> {
        let mut signatures = Vec::with_capacity(self.operations.len() + 8);
        signatures.push(format!("GetFunctions()({})", "函数列表"));
        signatures.push(format!("GetFrameworkVersion()({})", "框架版本"));
        signatures.push(format!("GetProjectVersion()({})", "项目版本"));
        signatures.push(format!(
            "GetInitConfig()({})",
            self.init_config_definition()
        ));
        signatures.push(format!(
            "GetPortNO({})(搜索结果,端口号)",
            definitions(self.port_probe_inputs)
        ));
        signatures.push(format!(
            "InitDevice({})({})",
            self.init_parameter_definition(),
            definitions(self.init_outputs)
        ));
        signatures.push("CloseDevice()()".to_owned());
        signatures.push("CloseAllDevices()()".to_owned());
        signatures.extend(self.operations.iter().map(OperationDescriptor::signature));
        signatures
    }

    /// 返回原始初始化配置声明，不展开自动候选；宿主查询使用 init_config_definitions_for。
    pub fn init_config_definitions(&self) -> Vec<String> {
        self.config_definitions()
    }

    fn config_definitions(&self) -> Vec<String> {
        let layout = ConfigLayout::new(
            self.init_config.len(),
            self.instances,
            self.lease_activation.has_platform_selector(),
        );
        let mut definitions = self
            .init_config
            .iter()
            .map(ParameterDescriptor::definition)
            .collect::<Vec<_>>();
        if layout.handle.is_some() {
            definitions.push(HANDLE_DEFINITION.to_owned());
        }
        if layout.lease.is_some() {
            definitions.push(RESOURCE_ACCESS_DEFINITION.to_owned());
        }
        if layout.execution {
            definitions.push(SHARED_EXECUTION_MODE_DEFINITION.to_owned());
        }
        definitions
    }

    fn init_config_definition(&self) -> String {
        self.init_config_definitions().join(",")
    }

    fn init_parameter_definition(&self) -> String {
        let mut definitions = self.init_config_definitions();
        definitions.extend(self.init_inputs.iter().map(ParameterDescriptor::definition));
        definitions.join(",")
    }
}

fn definitions(parameters: &[ParameterDescriptor]) -> String {
    parameters
        .iter()
        .map(ParameterDescriptor::definition)
        .collect::<Vec<_>>()
        .join(",")
}

fn definitions_for(
    parameters: &[ParameterDescriptor],
    adapter: &dyn crate::PlatformAdapter,
) -> crate::MiresidianResult<String> {
    Ok(parameters
        .iter()
        .map(|parameter| parameter.definition_for(adapter))
        .collect::<crate::MiresidianResult<Vec<_>>>()?
        .join(","))
}
