//! C ABI v1 的内存所有权、UTF-8 和错误状态回归测试。

use std::{
    ffi::{CStr, CString, c_char},
    ptr,
    sync::{Mutex, OnceLock},
};

use miresidian::{
    DeviceNode, InvocationOutput, MIRESIDIAN_STATUS_INTERNAL_ERROR,
    MIRESIDIAN_STATUS_INVALID_ARGUMENT, MIRESIDIAN_STATUS_OK, MiresidianError, MiresidianOutputV1,
    MiresidianResult, NodeDescriptor, NodeRuntime,
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

fn invoke(
    runtime: &OnceLock<Mutex<NodeRuntime<TestNode>>>,
    operation: *const c_char,
    output: *mut MiresidianOutputV1,
) -> i32 {
    // SAFETY: 每个测试都传入有效 NUL 结尾字符串或刻意传入空指针来验证 ABI 错误处理；
    // 输出指针在相应测试中有效，空指针用例只验证函数拒绝该输入。
    unsafe {
        miresidian::ffi_runtime::invoke_v1::<TestNode>(
            runtime,
            operation,
            ptr::null(),
            0,
            ptr::null(),
            0,
            output,
        )
    }
}

#[test]
fn 成功调用返回_utf8_输出且可重复释放() {
    let runtime = OnceLock::new();
    let operation = CString::new("GetFunctions").expect("静态文本不含 NUL");
    let mut output = MiresidianOutputV1::default();

    assert_eq!(
        invoke(&runtime, operation.as_ptr(), &mut output),
        MIRESIDIAN_STATUS_OK
    );
    assert!(!output.values.is_null());
    assert!(output.count >= 5);

    // SAFETY: 成功调用后 `values` 指向驱动分配的 NUL 结尾 UTF-8 字符串。
    let first = unsafe { CStr::from_ptr(*output.values) }
        .to_str()
        .expect("框架输出必须是 UTF-8");
    assert_eq!(first, "GetFunctions()(函数列表)");

    // SAFETY: 输出来自同一运行时的成功调用；释放函数会将其重置为默认值。
    unsafe { miresidian::ffi_runtime::free_output_v1(&mut output) };
    assert!(output.values.is_null());
    assert_eq!(output.count, 0);
    // SAFETY: 已重置为默认值的输出可再次释放，验证重复释放安全。
    unsafe { miresidian::ffi_runtime::free_output_v1(&mut output) };
}

#[test]
fn 无效_utf8_和空输出指针返回参数错误() {
    let runtime = OnceLock::new();
    let invalid_utf8 = [0xff_u8, 0];
    let mut output = MiresidianOutputV1::default();
    assert_eq!(
        invoke(&runtime, invalid_utf8.as_ptr().cast(), &mut output),
        MIRESIDIAN_STATUS_INVALID_ARGUMENT
    );
    assert!(last_error().contains("UTF-8"));

    let operation = CString::new("GetFunctions").expect("静态文本不含 NUL");
    assert_eq!(
        invoke(&runtime, operation.as_ptr(), ptr::null_mut()),
        MIRESIDIAN_STATUS_INVALID_ARGUMENT
    );
    assert!(last_error().contains("output"));
}

#[test]
fn panic_不会穿过_c_abi_边界() {
    let runtime = OnceLock::new();
    let operation = CString::new("InitDevice").expect("静态文本不含 NUL");
    let mut output = MiresidianOutputV1::default();
    assert_eq!(
        invoke(&runtime, operation.as_ptr(), &mut output),
        MIRESIDIAN_STATUS_OK
    );
    // SAFETY: 初始化成功后释放框架分配的空输出结构是合法的。
    unsafe { miresidian::ffi_runtime::free_output_v1(&mut output) };

    let operation = CString::new("Panic").expect("静态文本不含 NUL");
    assert_eq!(
        invoke(&runtime, operation.as_ptr(), &mut output),
        MIRESIDIAN_STATUS_INTERNAL_ERROR
    );
    assert!(last_error().contains("未预期错误"));
}

fn last_error() -> String {
    let pointer = miresidian::ffi_runtime::last_error_v1();
    assert!(!pointer.is_null());
    // SAFETY: 错误指针由当前线程本次 ABI 调用设置，读取发生在下一次 ABI 调用之前。
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .expect("框架错误必须为 UTF-8")
        .to_owned()
}
