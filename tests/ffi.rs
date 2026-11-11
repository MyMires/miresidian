//! JSON 文本 ABI 的固定操作回归测试。

use miresidian::{
    Invocation, InvocationOutput, MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK,
    MIRESIDIAN_STATUS_OPERATION_FAILED, MiresidianResult, Node, NodeDescriptor, NodeRuntime,
    Platform, PlatformAdapter, PlatformRegistry, RuntimeInvocation, RuntimeResponse,
};
use std::{
    ffi::{CStr, CString},
    sync::{Mutex, OnceLock},
};

static DESCRIPTOR: NodeDescriptor = NodeDescriptor {
    name: "FFI 测试节点",
    init_config: &[],
    init_inputs: &[],
    init_outputs: &[],
    port_probe_inputs: &[],
    operations: &[],
    lease_activation: miresidian::LeaseActivation::Disabled,
    instance: miresidian::InstancePolicy::DEFAULT,
};

struct TestNode;

impl Node for TestNode {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTOR
    }

    fn init_device(_: &[String], _: &[String]) -> MiresidianResult<(Self, InvocationOutput)> {
        Ok((Self, InvocationOutput::empty()))
    }

    fn close_device(&mut self, _: &[String], _: &[String]) -> MiresidianResult<()> {
        Ok(())
    }

    fn invoke(
        &mut self,
        _: &str,
        _: &[String],
        _: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::empty())
    }
}

struct CAbiPlatformNode;

impl Node for CAbiPlatformNode {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTOR
    }

    fn platform_registry() -> PlatformRegistry {
        PlatformRegistry::default().with_adapter(CAbiMockAdapter)
    }

    fn init_device(_: &[String], _: &[String]) -> MiresidianResult<(Self, InvocationOutput)> {
        Ok((Self, InvocationOutput::empty()))
    }

    fn close_device(&mut self, _: &[String], _: &[String]) -> MiresidianResult<()> {
        Ok(())
    }

    fn invoke(
        &mut self,
        _: &str,
        _: &[String],
        _: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::empty())
    }
}

#[derive(Clone, Copy)]
struct CAbiMockAdapter;

impl PlatformAdapter for CAbiMockAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(0x0001_0002)
    }

    fn identifier(&self) -> &'static str {
        "c_abi_mock_platform"
    }

    fn decode(&self, invocation: Invocation) -> MiresidianResult<RuntimeInvocation> {
        Ok(RuntimeInvocation::Single(invocation))
    }

    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(_) => Ok(InvocationOutput::new(vec!["c-abi-mock".to_owned()])),
            RuntimeResponse::MultiChannel(_) => {
                Ok(InvocationOutput::new(vec!["c-abi-mock-shared".to_owned()]))
            }
        }
    }
}

fn call_on<N: Node>(
    runtime: &OnceLock<Mutex<NodeRuntime<N>>>,
    request: serde_json::Value,
) -> serde_json::Value {
    let request = CString::new(request.to_string()).unwrap();
    // SAFETY: JSON 请求在调用期间有效，立即复制 DLL 响应。
    let output = unsafe { miresidian::ffi_runtime::invoke(runtime, request.as_ptr()) };
    serde_json::from_str(unsafe { CStr::from_ptr(output) }.to_str().unwrap()).unwrap()
}

fn call(function: &str, config: serde_json::Value) -> serde_json::Value {
    call_on(
        &OnceLock::<Mutex<NodeRuntime<TestNode>>>::new(),
        serde_json::json!({"function":function,"config":config}),
    )
}

#[test]
fn sole_json_entry_returns_metadata_without_platform_selector() {
    assert_eq!(
        call("GetFunctions", serde_json::json!([]))["output"][0],
        "GetFunctions()(函数列表)"
    );
    let output = call("GetInitConfig", serde_json::json!([]));
    assert_eq!(output["status"], MIRESIDIAN_STATUS_OK);
    assert_eq!(output["output"], serde_json::json!([]));
}

#[test]
fn default_node_probe_returns_exact_unsupported_reason() {
    assert_eq!(
        call("GetPortNO", serde_json::json!([])),
        serde_json::json!({
            "status": 0,
            "code": "OK",
            "message": "",
            "output": ["0", "当前未实现端口发现能力"],
        })
    );
}

#[test]
fn json_platform_selects_registered_adapter_and_rejects_reserved_ids() {
    let runtime = OnceLock::<Mutex<NodeRuntime<CAbiPlatformNode>>>::new();
    let initialized = call_on(
        &runtime,
        serde_json::json!({"platform":65538,"function":"InitDevice"}),
    );
    assert_eq!(initialized["status"], MIRESIDIAN_STATUS_OK);
    assert_eq!(initialized["output"], serde_json::json!(["c-abi-mock"]));
    let invalid = call_on(
        &runtime,
        serde_json::json!({"platform":2,"function":"GetFunctions"}),
    );
    assert_eq!(invalid["status"], MIRESIDIAN_STATUS_INVALID_ARGUMENT);
    assert_eq!(invalid["code"], "INVALID_PLATFORM_ID");
}

#[test]
fn config_platform_selection_is_rejected() {
    let output = call(
        "InitDevice",
        serde_json::json!(["平台适配:tb_lv_test_platform"]),
    );
    assert_eq!(output["status"], MIRESIDIAN_STATUS_INVALID_ARGUMENT);
    assert!(
        output["message"]
            .as_str()
            .unwrap()
            .contains("不应在 Config 中重复选择")
    );
}

#[test]
fn runtime_and_request_errors_preserve_status_categories() {
    assert_eq!(
        call("InitDevice", serde_json::json!(["unexpected", "extra"]))["status"],
        MIRESIDIAN_STATUS_INVALID_ARGUMENT
    );
    assert_eq!(
        call("UnknownOperation", serde_json::json!([]))["status"],
        MIRESIDIAN_STATUS_OPERATION_FAILED
    );
    let output = call("GetFunctions", serde_json::json!("not an array"));
    assert_eq!(output["status"], MIRESIDIAN_STATUS_INVALID_ARGUMENT);
    assert_eq!(output["output"], serde_json::json!([]));
}
