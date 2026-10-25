use miresidian::{
    CLOSE_ALL_DEVICES_OPERATION, CLOSE_DEVICE_OPERATION, DeviceNode,
    GET_FRAMEWORK_VERSION_OPERATION, GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION,
    GET_PORT_NO_OPERATION, GET_PROJECT_VERSION_OPERATION, INIT_DEVICE_OPERATION, Invocation,
    InvocationOutput, MIRESIDIAN_VERSION, MiresidianError, MiresidianResult, NodeDescriptor,
    NodeRuntime, OperationDescriptor, ParameterDescriptor, Platform, PlatformAdapter,
    PlatformRegistry, RuntimeInvocation, RuntimeResponse, SharedInvocation, TB_LV_TEST_PLATFORM,
    TbLvTestPlatformAdapter,
};

const INIT_CONFIG: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "地址",
    definition: "地址:127.0.0.1",
    default_value: Some("127.0.0.1"),
}];
const INIT_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "状态",
    definition: "状态",
    default_value: None,
}];
const QUERY_INPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "发送数据",
    definition: "发送数据",
    default_value: None,
}];
const QUERY_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "接收数据",
    definition: "接收数据",
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
    init_inputs: &[],
    init_outputs: &INIT_OUTPUTS,
    port_probe_inputs: &[],
    operations: &OPERATIONS,
    lease_activation: miresidian::LeaseActivation::Disabled,
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
        if self.address == "fail-close" {
            return Err(MiresidianError::Failure("模拟关闭失败".to_owned()));
        }
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

#[derive(Debug, Clone, Copy)]
struct MockPlatformAdapter;

impl PlatformAdapter for MockPlatformAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(0x0001_0000)
    }

    fn identifier(&self) -> &'static str {
        "mock_platform"
    }

    fn decode(&self, mut invocation: Invocation) -> MiresidianResult<RuntimeInvocation> {
        if invocation.operation == "MockQuery" {
            invocation.operation = "Query".to_owned();
        }
        Ok(RuntimeInvocation::Single(invocation))
    }

    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(output) => {
                let mut values = output.into_values();
                values.insert(0, "mock".to_owned());
                Ok(InvocationOutput::new(values))
            }
            RuntimeResponse::Shared(_) => Ok(InvocationOutput::new(vec!["mock-shared".to_owned()])),
        }
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
            .contains(&"InitDevice(地址:127.0.0.1,平台适配:tb_lv_test_platform,共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播)(状态)".to_owned())
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
    assert!(
        functions
            .as_slice()
            .contains(&"CloseAllDevices()()".to_owned())
    );

    let config = runtime
        .invoke(&request(GET_INIT_CONFIG_OPERATION, &[], &[]))
        .unwrap();
    assert_eq!(
        config.as_slice(),
        [
            "地址:127.0.0.1",
            "平台适配:tb_lv_test_platform",
            "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播",
        ]
    );
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
fn close_all_devices_releases_every_channel_and_resets_the_runtime() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["A"], &[]))
        .unwrap();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["B"], &[]))
        .unwrap();

    let output = runtime
        .invoke(&request(CLOSE_ALL_DEVICES_OPERATION, &[], &[]))
        .unwrap();
    assert!(output.as_slice().is_empty());
    assert!(!runtime.is_initialized());
    assert_eq!(
        runtime.invoke(&request("Query", &["A"], &["ping"])),
        Err(MiresidianError::NotInitialized)
    );

    let error = runtime
        .invoke(&request(CLOSE_ALL_DEVICES_OPERATION, &[], &["unexpected"]))
        .expect_err("批量关闭不接受业务参数");
    assert!(error.to_string().contains("不接受 Config 或 Input 参数"));
}

#[test]
fn shutdown_converges_the_runtime_after_a_close_failure() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["A"], &[]))
        .unwrap();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["fail-close"], &[]))
        .unwrap();

    let error = runtime
        .shutdown()
        .expect_err("节点关闭失败必须向调用方报告");
    assert!(error.to_string().contains("通道 [fail-close] 关闭失败"));
    assert!(!runtime.is_initialized());

    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["B"], &[]))
        .expect("失败清理后运行时必须可重新初始化");
}

#[test]
fn shutdown_resets_execution_and_platform_selection() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["A", "Parallel"], &[]))
        .unwrap();

    runtime.shutdown().expect("关闭应成功");
    assert!(!runtime.is_initialized());

    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["B", "Broadcast"], &[]))
        .expect("关闭后应能选择新的共享执行方式");
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
        .invoke(&request("Query", &[], &["共享", "one"]))
        .unwrap();
    assert_eq!(output.as_slice(), ["A:one", ""]);

    let extra_slot = runtime
        .invoke(&request("Query", &[], &["共享", "one", "two", "three"]))
        .expect_err("共享输入不得产生超过已初始化通道数的输出槽位");
    assert!(extra_slot.to_string().contains("超过已初始化通道数"));
}

#[test]
fn broadcast_uses_one_verified_plan_and_preserves_channel_outputs() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["A", "Broadcast"], &[]))
        .unwrap();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["B", "广播"], &[]))
        .unwrap();

    let output = runtime
        .invoke(&request("Query", &[], &["共享", "*IDN?", "*IDN?"]))
        .unwrap();
    assert_eq!(output.as_slice(), ["A:*IDN?", "B:*IDN?"]);

    let error = runtime
        .invoke(&request("Query", &[], &["共享", "first", "second"]))
        .expect_err("广播输入不一致必须拒绝");
    assert!(
        error
            .to_string()
            .contains("广播执行要求各通道 Input 参数完全一致")
    );
}

#[test]
fn invalid_or_blank_shared_execution_defaults_to_sequential() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["A", "未知方式"], &[]))
        .unwrap();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["B", ""], &[]))
        .unwrap();

    let output = runtime
        .invoke(&request("Query", &[], &["共享", "first", "second"]))
        .unwrap();
    assert_eq!(output.as_slice(), ["A:first", "B:second"]);
}

#[test]
fn 注册平台可由普通调用的初始化_config_选择() {
    let mut runtime = NodeRuntime::<MemoryNode>::with_platforms(
        PlatformRegistry::default().with_adapter(MockPlatformAdapter),
    );
    let metadata = runtime
        .invoke(&request(GET_INIT_CONFIG_OPERATION, &[], &[]))
        .unwrap();
    assert!(
        metadata
            .as_slice()
            .contains(&"平台适配:mock_platform/tb_lv_test_platform".to_owned())
    );

    let initialized = runtime
        .invoke(&request(
            INIT_DEVICE_OPERATION,
            &["A", "mock_platform", "Sequential"],
            &[],
        ))
        .unwrap();
    assert_eq!(initialized.as_slice(), ["mock", "已连接"]);

    let output = runtime
        .invoke(&request("MockQuery", &["A"], &["ping"]))
        .unwrap();
    assert_eq!(output.as_slice(), ["mock", "A:ping"]);
}

#[test]
fn 平台枚举可从注册表显式选择适配器() {
    let mut runtime = NodeRuntime::<MemoryNode>::with_platforms(
        PlatformRegistry::default().with_adapter(MockPlatformAdapter),
    );

    let initialized = runtime
        .invoke_with_platform(
            Platform::Custom(0x0001_0000),
            request(INIT_DEVICE_OPERATION, &["A", "Parallel"], &[]),
        )
        .unwrap();
    assert_eq!(initialized.as_slice(), ["mock", "已连接"]);
}

#[test]
fn 显式适配器调用不发布或接受平台选择_config() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    let config = runtime
        .invoke_with(
            &TbLvTestPlatformAdapter,
            request(GET_INIT_CONFIG_OPERATION, &[], &[]),
        )
        .unwrap();
    assert_eq!(
        config.as_slice(),
        [
            "地址:127.0.0.1",
            "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播",
        ]
    );

    runtime
        .invoke_with(
            &TbLvTestPlatformAdapter,
            request(INIT_DEVICE_OPERATION, &["A", "Parallel"], &[]),
        )
        .expect("显式适配器初始化只接受共享执行方式");

    let error = runtime
        .invoke_with(
            &TbLvTestPlatformAdapter,
            request(INIT_DEVICE_OPERATION, &["B", TB_LV_TEST_PLATFORM], &[]),
        )
        .expect_err("显式适配器路径不得再通过 Config 选择平台");
    assert!(error.to_string().contains("不应在 Config 中重复选择"));
}

#[test]
fn 结构化共享命令不依赖_tb_lv_test_platform_文本协议() {
    let mut runtime = NodeRuntime::<MemoryNode>::new();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["A"], &[]))
        .unwrap();
    runtime
        .invoke(&request(INIT_DEVICE_OPERATION, &["B"], &[]))
        .unwrap();

    let response = runtime
        .invoke_command(RuntimeInvocation::Shared(SharedInvocation::new(
            "Query",
            vec![Some(vec!["one".to_owned()])],
        )))
        .expect("结构化共享调用应由运行时执行");
    let RuntimeResponse::Shared(response) = response else {
        panic!("共享命令必须返回结构化共享结果");
    };
    assert_eq!(response.slots.len(), 2);
    assert_eq!(response.slots[0].as_ref().unwrap().as_slice(), ["A:one"]);
    assert!(response.slots[1].is_none());
}
