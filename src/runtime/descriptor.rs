//! 节点与操作的编译期元数据。

use super::instances::{ConfigLayout, HANDLE_DEFINITION, InstancePolicy};

pub use miresidian_instances::ExecutionMode;

/// 框架自动追加到 `InitDevice` 的多通道调度配置描述。
pub const SHARED_EXECUTION_MODE_DEFINITION: &str =
    "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播";

/// 框架自动追加到 `InitDevice` 的平台适配器选择器名称。
pub const PLATFORM_ADAPTER_NAME: &str = "平台适配";
/// 未显式构造平台注册表时发布的默认平台适配器。
pub const PLATFORM_ADAPTER_DEFINITION: &str = "平台适配:tb_lv_test_platform";

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
    /// 原样发布给平台的完整参数描述。
    pub definition: &'static str,
    /// 可选的平台侧默认值。
    pub default_value: Option<&'static str>,
}

impl ParameterDescriptor {
    /// 生成 C# 兼容的元数据形式：`名称` 或 `名称:默认值`。
    pub fn definition(&self) -> String {
        self.definition.to_owned()
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
    /// 生成兼容签名：`名称(输入...)(输出...)`。
    pub fn signature(&self) -> String {
        format!(
            "{}({})({})",
            self.name,
            definitions(self.inputs),
            definitions(self.outputs)
        )
    }
}

/// 设备节点的编译期元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeDescriptor {
    /// 用于诊断及后续发现机制的稳定节点名称。
    pub name: &'static str,
    /// 初始化业务 Config；Config 关联模式以首项为主键，框架保留字段不在此切片中。
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
    /// 返回固定操作和节点自定义操作的全部签名。
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

    /// 返回包含运行时平台注册表的平台选择器的全部签名。
    pub fn function_signatures_with_platform(&self, platform_selector: &str) -> Vec<String> {
        let mut signatures = Vec::with_capacity(self.operations.len() + 8);
        signatures.push(format!("GetFunctions()({})", "函数列表"));
        signatures.push(format!("GetFrameworkVersion()({})", "框架版本"));
        signatures.push(format!("GetProjectVersion()({})", "项目版本"));
        signatures.push(format!(
            "GetInitConfig()({})",
            self.init_config_definition_with_platform(platform_selector)
        ));
        signatures.push(format!(
            "GetPortNO({})(搜索结果,端口号)",
            definitions(self.port_probe_inputs)
        ));
        signatures.push(format!(
            "InitDevice({})({})",
            self.init_parameter_definition_with_platform(platform_selector),
            definitions(self.init_outputs)
        ));
        signatures.push("CloseDevice()()".to_owned());
        signatures.push("CloseAllDevices()()".to_owned());
        signatures.extend(self.operations.iter().map(OperationDescriptor::signature));
        signatures
    }

    /// 返回 `GetInitConfig` 所使用的初始化配置元数据。
    pub fn init_config_definitions(&self) -> Vec<String> {
        self.config_definitions(None)
    }

    /// 返回包含运行时平台注册表的平台选择器的初始化配置元数据。
    pub fn init_config_definitions_with_platform(&self, platform_selector: &str) -> Vec<String> {
        self.config_definitions(Some(platform_selector))
    }

    fn config_definitions(&self, platform_selector: Option<&str>) -> Vec<String> {
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
        if let Some(platform_selector) = platform_selector {
            definitions.push(platform_selector.to_owned());
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

    fn init_config_definition_with_platform(&self, platform_selector: &str) -> String {
        self.init_config_definitions_with_platform(platform_selector)
            .join(",")
    }

    /// 返回 `InitDevice` 签名所发布的全部参数。配置与输入在 ABI 中属于不同数组，
    /// 但函数签名需同时使平台可发现两类业务参数。
    fn init_parameter_definition_with_platform(&self, platform_selector: &str) -> String {
        let mut definitions = self.init_config_definitions_with_platform(platform_selector);
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
