//! 节点与操作的编译期元数据。

/// 配置值、输入值或输出值的元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterDescriptor {
    /// 展示给测试平台的名称。
    pub name: &'static str,
    /// 可选的平台侧默认值。
    pub default_value: Option<&'static str>,
}

impl ParameterDescriptor {
    /// 生成 C# 兼容的元数据形式：`名称` 或 `名称:默认值`。
    pub fn definition(&self) -> String {
        match self.default_value {
            Some(default_value) => format!("{}:{default_value}", self.name),
            None => self.name.to_owned(),
        }
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
            definitions(self.init_config)
        ));
        signatures.push(format!(
            "GetPortNO({})(搜索结果,端口号)",
            definitions(self.port_probe_inputs)
        ));
        signatures.push(format!(
            "InitDevice({})({})",
            definitions(self.init_config),
            definitions(self.init_outputs)
        ));
        signatures.push("CloseDevice()()".to_owned());
        signatures.extend(self.operations.iter().map(OperationDescriptor::signature));
        signatures
    }

    /// 返回 `GetInitConfig` 所使用的初始化配置元数据。
    pub fn init_config_definitions(&self) -> Vec<String> {
        self.init_config
            .iter()
            .map(ParameterDescriptor::definition)
            .collect()
    }
}

fn definitions(parameters: &[ParameterDescriptor]) -> String {
    parameters
        .iter()
        .map(ParameterDescriptor::definition)
        .collect::<Vec<_>>()
        .join(",")
}
