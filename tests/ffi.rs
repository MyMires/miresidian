//! DLL 字符串数组入口的固定操作回归测试。

use miresidian::{
    DeviceNode, InvocationOutput, MIRESIDIAN_STATUS_OK, MiresidianResult, NodeDescriptor,
    NodeRuntime,
};
use std::{
    ffi::{CStr, CString},
    ptr,
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
unsafe fn values(mut array: *mut *mut std::ffi::c_char) -> Vec<String> {
    let mut result = Vec::new();
    while !array.is_null() {
        let value = unsafe { *array };
        if value.is_null() {
            break;
        }
        result.push(
            unsafe { CStr::from_ptr(value) }
                .to_str()
                .expect("输出必须为 UTF-8")
                .to_owned(),
        );
        array = unsafe { array.add(1) };
    }
    result
}
fn call(function: &str) -> Vec<String> {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let function = CString::new(function).unwrap();
    let mut output = ptr::null_mut();
    let status = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            function.as_ptr(),
            ptr::null(),
            ptr::null(),
            &mut output,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_OK);
    unsafe { values(output) }
}
#[test]
fn 合并入口返回_get_functions() {
    let value = call("GetFunctions");
    assert_eq!(value[0], "0");
    assert_eq!(value[1], "");
    assert_eq!(value[2], "GetFunctions()(函数列表)");
}
#[test]
fn 合并入口返回_get_init_config() {
    let value = call("GetInitConfig");
    assert_eq!(value, vec!["0".to_owned(), "".to_owned()]);
}
#[test]
fn 分离入口返回_get_functions_与_get_init_config() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    for name in ["GetFunctions", "GetInitConfig"] {
        let function = CString::new(name).unwrap();
        let (mut output, mut error) = (ptr::null_mut(), ptr::null_mut());
        let status = unsafe {
            miresidian::ffi_runtime::invoke_with_error(
                &runtime,
                function.as_ptr(),
                ptr::null(),
                ptr::null(),
                &mut output,
                &mut error,
            )
        };
        assert_eq!(status, MIRESIDIAN_STATUS_OK);
        assert!(unsafe { values(error) }.is_empty());
        let values = unsafe { values(output) };
        if name == "GetFunctions" {
            assert!(!values.is_empty())
        } else {
            assert!(values.is_empty())
        }
    }
}
#[test]
fn 无错误入口返回_get_functions_与_get_init_config() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    for name in ["GetFunctions", "GetInitConfig"] {
        let function = CString::new(name).unwrap();
        let mut output = ptr::null_mut();
        let status = unsafe {
            miresidian::ffi_runtime::invoke_without_error(
                &runtime,
                function.as_ptr(),
                ptr::null(),
                ptr::null(),
                &mut output,
            )
        };
        assert_eq!(status, MIRESIDIAN_STATUS_OK);
        let values = unsafe { values(output) };
        if name == "GetFunctions" {
            assert!(!values.is_empty())
        } else {
            assert!(values.is_empty())
        }
    }
}
