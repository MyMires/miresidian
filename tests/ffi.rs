//! C ABI 三个调用入口的输出、错误与内存所有权回归测试。

use std::{
    ffi::{CStr, CString},
    ptr,
    sync::{Mutex, OnceLock},
};

use miresidian::{
    DeviceNode, InvocationOutput, MIRESIDIAN_STATUS_INTERNAL_ERROR,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK, MIRESIDIAN_STATUS_OPERATION_FAILED,
    MiresidianError, MiresidianResult, MiresidianTextArray, NodeDescriptor, NodeRuntime,
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

    fn init_device(
        &mut self,
        _config: &[String],
        _input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        Ok(InvocationOutput::empty())
    }

    fn close_device(&mut self, _config: &[String], _input: &[String]) -> MiresidianResult<()> {
        Ok(())
    }

    fn invoke(
        &mut self,
        operation: &str,
        _config: &[String],
        _input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        match operation {
            "Panic" => panic!("用于验证 ABI 防护"),
            _ => Err(MiresidianError::InvalidOperation(operation.to_owned())),
        }
    }
}

unsafe fn values(value: &MiresidianTextArray) -> Vec<String> {
    (0..value.count)
        .map(|index| {
            // SAFETY: 测试仅在入口成功分配后的 count 范围内读取 NUL 结尾 UTF-8 文本。
            unsafe { CStr::from_ptr(*value.values.add(index)) }
                .to_str()
                .expect("框架输出必须是 UTF-8")
                .to_owned()
        })
        .collect()
}

#[test]
fn 合并入口按固定顺序返回错误和业务输出() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let function = CString::new("GetFunctions").expect("静态文本不含 NUL");
    let mut output = MiresidianTextArray::default();

    // SAFETY: 传入的字符串及输出结构均满足 ABI 契约。
    let status = unsafe {
        miresidian::ffi_runtime::invoke(
            &runtime,
            function.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            &mut output,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_OK);
    let result = unsafe { values(&output) };
    assert_eq!(result[0], "0");
    assert_eq!(result[1], "");
    assert_eq!(result[2], "GetFunctions()(函数列表)");

    // SAFETY: 数组来自同一 DLL 的入口调用。
    unsafe { miresidian::ffi_runtime::free_text_array(&mut output) };
}

#[test]
fn 双通道入口将操作错误写入_error() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let mut output = MiresidianTextArray::default();
    let mut error = MiresidianTextArray::default();

    let init = CString::new("InitDevice").expect("静态文本不含 NUL");
    // SAFETY: 初始化调用的字符串及两个输出结构均满足 ABI 契约。
    let status = unsafe {
        miresidian::ffi_runtime::invoke_with_error(
            &runtime,
            init.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            &mut output,
            &mut error,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_OK);
    // SAFETY: 初始化调用返回的数组可安全重置。
    unsafe {
        miresidian::ffi_runtime::free_text_array(&mut output);
        miresidian::ffi_runtime::free_text_array(&mut error);
    }

    let function = CString::new("不存在的方法").expect("静态文本不含 NUL");

    // SAFETY: 传入的字符串及两个输出结构均满足 ABI 契约。
    let status = unsafe {
        miresidian::ffi_runtime::invoke_with_error(
            &runtime,
            function.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            &mut output,
            &mut error,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_OPERATION_FAILED);
    assert_eq!(unsafe { values(&output) }, Vec::<String>::new());
    let error_values = unsafe { values(&error) };
    assert_eq!(error_values[0], "2");
    assert!(error_values[1].contains("不存在的方法"));

    // SAFETY: 两个数组均来自同一 DLL 的入口调用。
    unsafe {
        miresidian::ffi_runtime::free_text_array(&mut output);
        miresidian::ffi_runtime::free_text_array(&mut error);
    }
}

#[test]
fn 无错误入口不污染业务输出() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let function = CString::new("不存在的方法").expect("静态文本不含 NUL");
    let mut output = MiresidianTextArray::default();

    // SAFETY: 传入的字符串及输出结构均满足 ABI 契约。
    let status = unsafe {
        miresidian::ffi_runtime::invoke_without_error(
            &runtime,
            function.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            &mut output,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_OPERATION_FAILED);
    assert_eq!(unsafe { values(&output) }, Vec::<String>::new());
    // SAFETY: 零长度数组仍可安全释放。
    unsafe { miresidian::ffi_runtime::free_text_array(&mut output) };
}

#[test]
fn 参数错误和_panic_会通过对应通道返回() {
    let runtime: OnceLock<Mutex<NodeRuntime<TestNode>>> = OnceLock::new();
    let invalid_utf8 = [0xff_u8, 0];
    let mut output = MiresidianTextArray::default();
    let mut error = MiresidianTextArray::default();
    // SAFETY: 无效 UTF-8 用例仅验证入口错误处理，其他指针有效。
    let status = unsafe {
        miresidian::ffi_runtime::invoke_with_error(
            &runtime,
            invalid_utf8.as_ptr().cast(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            &mut output,
            &mut error,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_INVALID_ARGUMENT);
    assert!(unsafe { values(&error) }[1].contains("UTF-8"));
    // SAFETY: 为下一次调用重置两个框架分配的数组。
    unsafe {
        miresidian::ffi_runtime::free_text_array(&mut output);
        miresidian::ffi_runtime::free_text_array(&mut error);
    }

    let init = CString::new("InitDevice").expect("静态文本不含 NUL");
    // SAFETY: 初始化调用的字符串及输出结构均满足 ABI 契约。
    let status = unsafe {
        miresidian::ffi_runtime::invoke_with_error(
            &runtime,
            init.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            &mut output,
            &mut error,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_OK);
    // SAFETY: 初始化调用返回的两个数组均可安全重置。
    unsafe {
        miresidian::ffi_runtime::free_text_array(&mut output);
        miresidian::ffi_runtime::free_text_array(&mut error);
    }

    let function = CString::new("Panic").expect("静态文本不含 NUL");
    // SAFETY: 传入的字符串及输出结构均满足 ABI 契约。
    let status = unsafe {
        miresidian::ffi_runtime::invoke_with_error(
            &runtime,
            function.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            &mut output,
            &mut error,
        )
    };
    assert_eq!(status, MIRESIDIAN_STATUS_INTERNAL_ERROR);
    assert!(unsafe { values(&error) }[1].contains("未预期错误"));
    // SAFETY: 两个数组均来自同一 DLL 的入口调用。
    unsafe {
        miresidian::ffi_runtime::free_text_array(&mut output);
        miresidian::ffi_runtime::free_text_array(&mut error);
    }
}
