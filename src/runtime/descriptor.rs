//! 节点与操作的编译期元数据。

/// 共享调用在多通道上的执行方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// 在当前线程上按通道顺序依次执行。
    Sequential,
    /// 为各通道分发独立线程执行；同一对象仍互斥执行。
    Parallel,
    /// 复用同一套输入参数计划，按通道顺序广播执行。
    Broadcast,
}

impl ExecutionMode {
    /// 共享执行方式的稳定平台实际值。
    pub fn definition(&self) -> &'static str {
        match self {
            Self::Sequential => "Sequential",
            Self::Parallel => "Parallel",
            Self::Broadcast => "Broadcast",
        }
    }

    /// 解析平台传入的实际值或显示值；空白和未知值安全回退为顺序执行。
    pub fn parse_platform_value(value: Option<&str>) -> Self {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "parallel" | "并行" => Self::Parallel,
            "broadcast" | "广播" => Self::Broadcast,
            "sequential" | "顺序" => Self::Sequential,
            _ => Self::Sequential,
        }
    }
}

/// 框架自动追加到 `InitDevice` 的共享调度配置描述。
pub const SHARED_EXECUTION_MODE_DEFINITION: &str =
    "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播";

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
    /// 保留的操作声明执行方式；共享调用实际采用运行时初始化配置。
    pub execution: ExecutionMode,
    /// 共享调用时用于判定通道参数串合法性的 Input 来源参数；不含 `config` 标注参数。
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
    /// `InitDevice` 的有序配置值。
    pub init_config: &'static [ParameterDescriptor],
    /// `InitDevice` 的有序输出。
    pub init_outputs: &'static [ParameterDescriptor],
    /// `GetPortNO` 的有序输入；为空表示该驱动不需要探测输入。
    pub port_probe_inputs: &'static [ParameterDescriptor],
    /// 节点自定义的平台操作。
    pub operations: &'static [OperationDescriptor],
    /// 租约启用策略；由 `#[node(..., lease(...))]` 声明。
    pub lease_activation: LeaseActivation,
}

impl NodeDescriptor {
    /// 返回固定操作和节点自定义操作的全部签名。
    pub fn function_signatures(&self) -> Vec<String> {
        let mut signatures = Vec::with_capacity(self.operations.len() + 7);
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
            self.init_config_definition(),
            definitions(self.init_outputs)
        ));
        signatures.push("CloseDevice()()".to_owned());
        signatures.extend(self.operations.iter().map(OperationDescriptor::signature));
        signatures
    }

    /// 返回 `GetInitConfig` 所使用的初始化配置元数据。
    pub fn init_config_definitions(&self) -> Vec<String> {
        let mut definitions = self
            .init_config
            .iter()
            .map(ParameterDescriptor::definition)
            .collect::<Vec<_>>();
        if self.lease_activation.has_platform_selector() {
            definitions.push(RESOURCE_ACCESS_DEFINITION.to_owned());
        }
        definitions.push(SHARED_EXECUTION_MODE_DEFINITION.to_owned());
        definitions
    }

    fn init_config_definition(&self) -> String {
        self.init_config_definitions().join(",")
    }
}

fn definitions(parameters: &[ParameterDescriptor]) -> String {
    parameters
        .iter()
        .map(ParameterDescriptor::definition)
        .collect::<Vec<_>>()
        .join(",")
}
