//! JSON 文本 ABI 的固定操作回归测试。

use miresidian::{
    DeviceNode, InvocationOutput, MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK,
    MiresidianResult, NodeDescriptor, NodeRuntime,
};
use std::{
    ffi::CString,
    sync::{Mutex, OnceLock},
};

const OUTPUT_CAPACITY: u32 = 4_096;

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

fn call(function: &str) -> (i32, serde_json::Value) {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let function = CString::new(function).expect("固定操作名不含 NUL");
    let mut output = vec![0_i8; OUTPUT_CAPACITY as usize];
    let status = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            function.as_ptr(),
            c"[]".as_ptr(),
            c"[]".as_ptr(),
            output.as_mut_ptr(),
            OUTPUT_CAPACITY,
        )
    };
    let output = unsafe { std::ffi::CStr::from_ptr(output.as_ptr()) }
        .to_str()
        .expect("输出必须为 UTF-8");
    let output = serde_json::from_str(output).expect("输出必须为 JSON 响应对象");
    (status, output)
}

#[test]
fn 合并入口返回_get_functions_json() {
    let (status, output) = call("GetFunctions");
    assert_eq!(status, MIRESIDIAN_STATUS_OK);
    assert_eq!(output["error"], serde_json::Value::Null);
    assert_eq!(output["output"][0], "GetFunctions()(函数列表)");
}

#[test]
fn 合并入口返回_get_init_config_json() {
    let (status, output) = call("GetInitConfig");
    assert_eq!(status, MIRESIDIAN_STATUS_OK);
    assert_eq!(output["error"], serde_json::Value::Null);
    assert_eq!(output["output"], serde_json::json!([]));
}

#[test]
fn 非法_json_返回参数错误() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let function = c"GetFunctions";
    let invalid_json = c"[";
    let mut output = vec![0_i8; OUTPUT_CAPACITY as usize];
    let status = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            function.as_ptr(),
            invalid_json.as_ptr(),
            c"[]".as_ptr(),
            output.as_mut_ptr(),
            OUTPUT_CAPACITY,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_INVALID_ARGUMENT);
    let output = unsafe { std::ffi::CStr::from_ptr(output.as_ptr()) }
        .to_str()
        .expect("输出必须为 UTF-8");
    let output: serde_json::Value = serde_json::from_str(output).expect("输出必须为 JSON 响应对象");
    assert_eq!(output["output"], serde_json::json!([]));
    assert!(
        output["error"]
            .as_str()
            .expect("错误信息必须是字符串")
            .contains("config_json")
    );
}

#[test]
fn 输出缓冲区不足不会越界写入() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let mut output = [0_i8; 2];
    let status = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            c"GetFunctions".as_ptr(),
            c"[]".as_ptr(),
            c"[]".as_ptr(),
            output.as_mut_ptr(),
            output.len() as u32,
        )
    };
    assert_eq!(
        status,
        miresidian::ffi_runtime::MIRESIDIAN_STATUS_OUTPUT_BUFFER_TOO_SMALL
    );
    assert_eq!(output, [0, 0]);
}
