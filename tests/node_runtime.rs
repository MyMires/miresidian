use miresidian::{
    CLOSE_DEVICE_OPERATION, DeviceNode, GET_FRAMEWORK_VERSION_OPERATION, GET_FUNCTIONS_OPERATION,
    GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION, GET_PROJECT_VERSION_OPERATION,
    INIT_DEVICE_OPERATION, Invocation, InvocationOutput, MIRESIDIAN_VERSION, MiresidianError,
    MiresidianResult, NodeDescriptor, NodeRuntime, OperationDescriptor, ParameterDescriptor,
};

const INIT_CONFIG: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "地址",
    default_value: Some("127.0.0.1"),
}];
const INIT_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "状态",
    default_value: None,
}];
const QUERY_INPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "发送数据",
    default_value: None,
}];
const QUERY_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "接收数据",
    default_value: None,
}];
const OPERATIONS: [OperationDescriptor; 1] = [OperationDescriptor {
    name: "Query",
    inputs: &QUERY_INPUTS,
    outputs: &QUERY_OUTPUTS,
    execution: miresidian::ExecutionMode::Sequential,
    channel_inputs: &QUERY_INPUTS,
}];
const DESCRIPTOR: NodeDescriptor = NodeDescriptor {
    name: "MemoryNode",
    init_config: &INIT_CONFIG,
    init_outputs: &INIT_OUTPUTS,
    port_probe_inputs: &[],
    operations: &OPERATIONS,
};

#[derive(Default)]
struct MemoryNode {
    address: String,
}

impl DeviceNode for MemoryNode {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTOR
    }

    fn project_version() -> &'static str {
        "9.8.7"
    }
    fn init_device(
        &mut self,
        config: &[String],
        _input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        if config.len() != 1 {
            return Err(MiresidianError::InvalidArgument {
                field: "地址".to_owned(),
                message: "需要一个配置值".to_owned(),
            });
        }
        self.address = config[0].clone();
        Ok(InvocationOutput::new(vec!["已连接".to_owned()]))
    }

    fn close_device(&mut self, _config: &[String], _input: &[String]) -> MiresidianResult<()> {
        Ok(())
    }

    fn invoke(
        &mut self,
        operation: &str,
        _config: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        match operation {
            "Query" if input.len() == 1 => Ok(InvocationOutput::new(vec![format!(
                "{}:{}",
                self.address, input[0]
            )])),
            "Query" => Err(MiresidianError::InvalidArgument {
                field: "发送数据".to_owned(),
                message: "需要一个输入值".to_owned(),
            }),
            other => Err(MiresidianError::InvalidOperation(other.to_owned())),
        }
    }

    fn get_port_no(
        &mut self,
        _config: &[String],
        _input: &[String],
    ) -> MiresidianResult<Option<String>> {
        Ok(Some("COM7".to_owned()))
    }
}

fn request(operation: &str, config: &[&str], input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
        input
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
    )
}

#[test]
fn fixed_operations_publish_compatible_metadata() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();

    let functions = runtime
        .invoke(&request(GET_FUNCTIONS_OPERATION, &[], &[]))
        .unwrap();
    assert!(
        functions
            .as_slice()
            .contains(&"InitDevice(地址:127.0.0.1)(状态)".to_owned())
    );
    assert!(
        functions
            .as_slice()
            .contains(&"Query(发送数据)(接收数据)".to_owned())
    );
    assert!(
        functions
            .as_slice()
            .contains(&"GetPortNO()(搜索结果,端口号)".to_owned())
    );

    let config = runtime
        .invoke(&request(GET_INIT_CONFIG_OPERATION, &[], &[]))
        .unwrap();
    assert_eq!(config.as_slice(), ["地址:127.0.0.1"]);
}

#[test]
fn version_queries_are_persistent() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();

    let framework = runtime
        .invoke(&request(GET_FRAMEWORK_VERSION_OPERATION, &[], &[]))
        .unwrap();
    assert_eq!(framework.as_slice(), [MIRESIDIAN_VERSION]);

    let project = runtime
        .invoke(&request(GET_PROJECT_VERSION_OPERATION, &[], &[]))
        .unwrap();
    assert_eq!(project.as_slice(), ["9.8.7"]);
    assert!(!runtime.is_initialized());
}

#[test]
fn node_operations_require_lifecycle_and_round_trip_values() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();

    assert_eq!(
        runtime.invoke(&request("Query", &["127.0.0.1"], &["ping"])),
        Err(MiresidianError::NotInitialized)
    );

    let init = runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["127.0.0.1"], &[]))
        .unwrap();
    assert_eq!(init.as_slice(), ["已连接"]);
    assert!(runtime.is_initialized());

    let query = runtime
        .invoke(&request("Query", &["127.0.0.1"], &["ping"]))
        .unwrap();
    assert_eq!(query.as_slice(), ["127.0.0.1:ping"]);

    runtime
        .invoke(&request(CLOSE_DEVICE_OPERATION, &["127.0.0.1"], &[]))
        .unwrap();
    assert!(!runtime.is_initialized());
}

#[test]
fn port_probe_uses_a_temporary_node() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();

    let result = runtime
        .invoke(&request(GET_PORT_NO_OPERATION, &[], &[]))
        .unwrap();

    assert_eq!(result.as_slice(), ["1", "COM7"]);
    assert!(!runtime.is_initialized());
}

#[test]
fn config_keys_create_reuse_and_shared_dispatches_each_channel() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    assert_eq!(
        runtime
            .invoke(&request(INIT_DEVICE_OPERATION, &["A"], &[]))
            .unwrap()
            .as_slice(),
        ["已连接"]
    );
    assert_eq!(
        runtime
            .invoke(&request(INIT_DEVICE_OPERATION, &["B"], &[]))
            .unwrap()
            .as_slice(),
        ["已连接"]
    );
    assert_eq!(
        runtime
            .invoke(&request(INIT_DEVICE_OPERATION, &["A"], &[]))
            .unwrap()
            .as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );

    let output = runtime
        .invoke(&request("Query", &[], &["共享", "one", "  ", "three"]))
        .unwrap();
    assert_eq!(output.as_slice(), ["A:one", "", ""]);
}
