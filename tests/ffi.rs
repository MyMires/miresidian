//! JSON 文本 ABI 的固定操作回归测试。

use miresidian::{
    DeviceNode, InvocationOutput, MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK,
    MiresidianResult, NodeDescriptor, NodeRuntime,
};
use std::{
    ffi::{CStr, CString},
    sync::{Mutex, OnceLock},
};

static DESCRIPTOR: NodeDescriptor = NodeDescriptor {
    name: "FFI 测试节点",
    init_config: &[],
    init_outputs: &[],
    port_probe_inputs: &[],
    operations: &[],
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
    assert_eq!(output["output"], serde_json::json!([]));
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
