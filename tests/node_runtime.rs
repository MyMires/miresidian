use miresidian::{
    CLOSE_DEVICE_OPERATION, DeviceNode, DriverError, DriverResult, GET_FUNCTIONS_OPERATION,
    GET_INIT_CONFIG_OPERATION, GET_PORT_NO_OPERATION, INIT_DEVICE_OPERATION, Invocation,
    InvocationOutput, NodeDescriptor, NodeRuntime, OperationDescriptor, ParameterDescriptor,
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
}];
const DESCRIPTOR: NodeDescriptor = NodeDescriptor {
    name: "MemoryNode",
    init_config: &INIT_CONFIG,
    init_outputs: &INIT_OUTPUTS,
    port_probe_inputs: &[],
    operations: &OPERATIONS,
};

#[derive(Default)]
struct MemoryNode;

impl DeviceNode for MemoryNode {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTOR
    }

    fn init_device(
        &mut self,
        config: &[String],
        _input: &[String],
    ) -> DriverResult<InvocationOutput> {
        if config.len() != 1 {
            return Err(DriverError::InvalidArgument {
                field: "地址".to_owned(),
                message: "需要一个配置值".to_owned(),
            });
        }
        Ok(InvocationOutput::new(vec!["已连接".to_owned()]))
    }

    fn close_device(&mut self, _config: &[String], _input: &[String]) -> DriverResult<()> {
        Ok(())
    }

    fn invoke(
        &mut self,
        operation: &str,
        _config: &[String],
        input: &[String],
    ) -> DriverResult<InvocationOutput> {
        match operation {
            "Query" if input.len() == 1 => Ok(InvocationOutput::new(vec![input[0].clone()])),
            "Query" => Err(DriverError::InvalidArgument {
                field: "发送数据".to_owned(),
                message: "需要一个输入值".to_owned(),
            }),
            other => Err(DriverError::InvalidOperation(other.to_owned())),
        }
    }

    fn get_port_no(
        &mut self,
        _config: &[String],
        _input: &[String],
    ) -> DriverResult<Option<String>> {
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
fn node_operations_require_lifecycle_and_round_trip_values() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();

    assert_eq!(
        runtime.invoke(&request("Query", &[], &["ping"])),
        Err(DriverError::NotInitialized)
    );

    let init = runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["127.0.0.1"], &[]))
        .unwrap();
    assert_eq!(init.as_slice(), ["已连接"]);
    assert!(runtime.is_initialized());

    let query = runtime.invoke(&request("Query", &[], &["ping"])).unwrap();
    assert_eq!(query.as_slice(), ["ping"]);

    runtime
        .invoke(&request(CLOSE_DEVICE_OPERATION, &[], &[]))
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
