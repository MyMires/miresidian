//! JSON 文本 ABI 的固定操作回归测试。

use miresidian::{
    DeviceNode, Invocation, InvocationOutput, MIRESIDIAN_STATUS_INVALID_ARGUMENT,
    MIRESIDIAN_STATUS_OK, MiresidianResult, NodeDescriptor, NodeRuntime, PlatformAdapter,
    PlatformRegistry, RuntimeInvocation, RuntimeResponse,
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
};

#[derive(Default)]
struct TestNode;

impl DeviceNode for TestNode {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTOR
    }

    fn init_device(&mut self, _: &[String], _: &[String]) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::empty())
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

#[derive(Default)]
struct CAbiPlatformNode;

impl DeviceNode for CAbiPlatformNode {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTOR
    }

    fn platform_registry() -> PlatformRegistry {
        PlatformRegistry::default().with_adapter(CAbiMockAdapter)
    }

    fn init_device(&mut self, _: &[String], _: &[String]) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::empty())
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
    fn identifier(&self) -> &'static str {
        "c_abi_mock_platform"
    }

    fn decode(&self, invocation: Invocation) -> MiresidianResult<RuntimeInvocation> {
        Ok(RuntimeInvocation::Single(invocation))
    }

    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(_) => Ok(InvocationOutput::new(vec!["c-abi-mock".to_owned()])),
            RuntimeResponse::Shared(_) => {
                Ok(InvocationOutput::new(vec!["c-abi-mock-shared".to_owned()]))
            }
        }
    }
}

fn call(function: &str, config_json: &str) -> serde_json::Value {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let function = CString::new(function).expect("固定操作名不含 NUL");
    let config_json = CString::new(config_json).expect("JSON 不含 NUL");
    let output = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            function.as_ptr(),
            config_json.as_ptr(),
            c"[]".as_ptr(),
        )
    };
    let output = unsafe { CStr::from_ptr(output) }
        .to_str()
        .expect("输出必须为 UTF-8");
    serde_json::from_str(output).expect("输出必须为 JSON 响应对象")
}

#[test]
fn get_functions_直接返回_json_文本() {
    let output = call("GetFunctions", "[]");
    assert_eq!(output["status"], MIRESIDIAN_STATUS_OK);
    assert_eq!(output["message"], "");
    assert_eq!(output["output"][0], "GetFunctions()(函数列表)");
}

#[test]
fn get_init_config_无需调用方输出缓冲区() {
    let output = call("GetInitConfig", "[]");
    assert_eq!(output["status"], MIRESIDIAN_STATUS_OK);
    assert_eq!(output["message"], "");
    assert_eq!(
        output["output"],
        serde_json::json!([
            "平台适配:tb_lv_test_platform",
            "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播"
        ])
    );
}

#[test]
fn c_abi_可从节点注册表选择编译进_dll_的平台() {
    let runtime: OnceLock<Mutex<NodeRuntime<CAbiPlatformNode>>> = OnceLock::new();
    let function = CString::new("GetInitConfig").unwrap();
    let config = CString::new("[]").unwrap();
    let metadata = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            function.as_ptr(),
            config.as_ptr(),
            c"[]".as_ptr(),
        )
    };
    let metadata: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(metadata) }.to_str().unwrap()).unwrap();
    assert!(
        metadata["output"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "平台适配:c_abi_mock_platform/tb_lv_test_platform"
            ))
    );

    let function = CString::new("InitDevice").unwrap();
    let config = CString::new("[\"c_abi_mock_platform\"]").unwrap();
    let initialized = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            function.as_ptr(),
            config.as_ptr(),
            c"[]".as_ptr(),
        )
    };
    let initialized: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(initialized) }.to_str().unwrap()).unwrap();
    assert_eq!(initialized["status"], MIRESIDIAN_STATUS_OK);
    assert_eq!(initialized["output"], serde_json::json!(["c-abi-mock"]));
}

#[test]
fn 非法_json_返回状态与消息() {
    let output = call("GetFunctions", "[");
    assert_eq!(output["status"], MIRESIDIAN_STATUS_INVALID_ARGUMENT);
    assert!(
        output["message"]
            .as_str()
            .expect("错误信息必须是字符串")
            .contains("config_json")
    );
    assert_eq!(output["output"], serde_json::json!([]));
}
