//! 经两类 C ABI 入口验证固定响应字段、稳定错误码和业务输出。
use miresidian::{
    Invocation, InvocationOutput, MiresidianError, MiresidianResult, NodeRuntime, Platform,
    PlatformAdapter, RuntimeInvocation, RuntimeResponse,
};
use std::{
    ffi::{CStr, CString, c_char},
    sync::{Mutex, OnceLock},
};

#[derive(Default)]
struct DiagnosticAdapter;

impl PlatformAdapter for DiagnosticAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(65_537)
    }
    fn identifier(&self) -> &'static str {
        "diagnostic"
    }
    fn decode(&self, request: Invocation) -> MiresidianResult<RuntimeInvocation> {
        assert_ne!(request.operation, "框架异常", "模拟框架侧异常");
        Ok(RuntimeInvocation::Single(request))
    }
    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(output) => Ok(output),
            RuntimeResponse::Shared(_) => unreachable!("此测试只使用单通道命令"),
        }
    }
}

#[miresidian::node("响应契约测试", platforms(DiagnosticAdapter))]
struct ContractNode;

impl ContractNode {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("地址")] address: String) -> MiresidianResult<Self> {
        let _ = address;
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation("数值")]
    fn number(&mut self, #[miresidian::input("值")] value: u32) -> MiresidianResult<u32> {
        Ok(value)
    }
    #[miresidian::operation("回显")]
    fn echo(&mut self, #[miresidian::input("文本")] text: String) -> MiresidianResult<String> {
        Ok(text)
    }
    #[miresidian::operation("失败")]
    fn fail(&mut self) -> MiresidianResult<()> {
        Err(MiresidianError::Failure("业务诊断可变化".to_owned()))
    }
    #[miresidian::operation("不支持")]
    fn unsupported(&mut self) -> MiresidianResult<()> {
        Err(MiresidianError::NotSupported("可选能力".to_owned()))
    }
    #[miresidian::operation("节点异常")]
    fn panic(&mut self) -> MiresidianResult<()> {
        panic!("模拟节点异常")
    }
}

type Runtime = OnceLock<Mutex<NodeRuntime<ContractNode>>>;

fn call(
    runtime: &Runtime,
    platform: Option<u32>,
    operation: &str,
    config: &[&str],
    input: &[&str],
) -> serde_json::Value {
    let operation = CString::new(operation).unwrap();
    let config = CString::new(serde_json::to_string(config).unwrap()).unwrap();
    let input = CString::new(serde_json::to_string(input).unwrap()).unwrap();
    // SAFETY: 所有输入 C 字符串存活至调用结束，响应立即复制。
    let response = unsafe {
        raw_call(
            runtime,
            platform,
            operation.as_ptr(),
            config.as_ptr(),
            input.as_ptr(),
        )
    };
    check_schema(&response)
}

unsafe fn raw_call(
    runtime: &Runtime,
    platform: Option<u32>,
    operation: *const c_char,
    config: *const c_char,
    input: *const c_char,
) -> String {
    // SAFETY: 调用者保证指针指向有效 C 字符串，或为契约中允许的 NULL。
    let response = unsafe {
        match platform {
            Some(platform) => {
                miresidian::ffi_runtime::invoke_with(runtime, platform, operation, config, input)
            }
            None => miresidian::ffi_runtime::invoke(runtime, operation, config, input),
        }
    };
    // SAFETY: ABI 返回线程本地有效 C 字符串，在下次调用前复制。
    unsafe { CStr::from_ptr(response) }
        .to_str()
        .unwrap()
        .to_owned()
}

fn check_schema(raw: &str) -> serde_json::Value {
    let response: serde_json::Value = serde_json::from_str(raw).unwrap();
    let object = response.as_object().unwrap();
    assert_eq!(object.len(), 4);
    assert!(response["status"].is_i64());
    assert!(response["code"].is_string());
    assert!(response["message"].is_string());
    assert!(
        response["output"]
            .as_array()
            .unwrap()
            .iter()
            .all(serde_json::Value::is_string)
    );
    assert!(raw.starts_with(&format!(
        "{{\"status\":{},\"code\":{},\"message\":",
        response["status"], response["code"]
    )));
    response
}

fn expect(response: &serde_json::Value, status: i32, code: &str) {
    assert_eq!(response["status"], status, "{response}");
    assert_eq!(response["code"], code, "{response}");
    if status == 0 {
        assert_eq!(response["message"], "");
    } else {
        assert!(!response["message"].as_str().unwrap().is_empty());
        assert_eq!(response["output"], serde_json::json!([]));
    }
}

#[test]
fn both_entries_preserve_version_queries_and_output_with_code() {
    for platform in [None, Some(1)] {
        let runtime = Runtime::new();
        let framework = call(&runtime, platform, "GetFrameworkVersion", &[], &[]);
        expect(&framework, 0, "OK");
        assert_eq!(
            framework["output"],
            serde_json::json!([miresidian::MIRESIDIAN_VERSION])
        );
        let project = call(&runtime, platform, "GetProjectVersion", &[], &[]);
        expect(&project, 0, "OK");
        assert_eq!(
            project["output"],
            serde_json::json!([env!("CARGO_PKG_VERSION")])
        );
        expect(
            &call(&runtime, platform, "InitDevice", &["A"], &[]),
            0,
            "OK",
        );
        let echoed = call(&runtime, platform, "回显", &["A"], &["中文,引号\"\n\0"]);
        expect(&echoed, 0, "OK");
        assert_eq!(echoed["output"], serde_json::json!(["中文,引号\"\n\0"]));
    }
}

#[test]
fn both_entries_report_stable_lifecycle_and_business_codes() {
    for platform in [None, Some(1)] {
        let runtime = Runtime::new();
        expect(
            &call(&runtime, platform, "数值", &["A"], &["1"]),
            2,
            "NOT_INITIALIZED",
        );
        expect(
            &call(&runtime, platform, "InitDevice", &["A"], &[]),
            0,
            "OK",
        );
        for (operation, config, input, status, code) in [
            ("数值", "B", vec!["1"], 2, "UNKNOWN_CHANNEL"),
            ("未声明操作", "A", vec![], 1, "INVALID_OPERATION"),
            ("数值", "A", vec!["不是整数"], 1, "INVALID_ARGUMENT"),
            ("失败", "A", vec![], 2, "OPERATION_FAILED"),
            ("不支持", "A", vec![], 2, "NOT_SUPPORTED"),
            ("节点异常", "A", vec![], 3, "NODE_PANICKED"),
        ] {
            expect(
                &call(&runtime, platform, operation, &[config], &input),
                status,
                code,
            );
        }
        expect(
            &call(&runtime, platform, "CloseDevice", &["A"], &[]),
            0,
            "OK",
        );
    }
}

#[test]
fn unknown_platform_is_distinct_from_reserved_platform_number() {
    let runtime = Runtime::new();
    expect(
        &call(
            &runtime,
            None,
            "InitDevice",
            &["A", "平台适配:missing"],
            &[],
        ),
        1,
        "UNKNOWN_PLATFORM_ADAPTER",
    );
    for platform in [0, 2, 65_535] {
        expect(
            &call(&runtime, Some(platform), "GetFunctions", &[], &[]),
            1,
            "INVALID_PLATFORM_ID",
        );
    }
    expect(
        &call(&runtime, Some(65_538), "GetFunctions", &[], &[]),
        1,
        "UNKNOWN_PLATFORM_ADAPTER",
    );
}

#[test]
fn malformed_abi_inputs_keep_the_same_four_fields() {
    for platform in [None, Some(1)] {
        let runtime = Runtime::new();
        let invalid_utf8 = CStr::from_bytes_with_nul(&[0xff, 0]).unwrap();
        for (function, config, input) in [
            (std::ptr::null(), c"[]".as_ptr(), c"[]".as_ptr()),
            (invalid_utf8.as_ptr(), c"[]".as_ptr(), c"[]".as_ptr()),
            (c"GetFunctions".as_ptr(), c"[".as_ptr(), c"[]".as_ptr()),
            (c"GetFunctions".as_ptr(), c"[]".as_ptr(), c"[1]".as_ptr()),
        ] {
            // SAFETY: 每个非空指针都指向有效 NUL 结尾缓冲区；非法 UTF-8 由 ABI 返回错误。
            let raw = unsafe { raw_call(&runtime, platform, function, config, input) };
            expect(&check_schema(&raw), 1, "INVALID_ARGUMENT");
        }
    }
}

#[test]
fn shared_context_preserves_error_category_and_code() {
    let runtime = Runtime::new();
    expect(&call(&runtime, None, "InitDevice", &["A"], &[]), 0, "OK");
    let response = call(&runtime, None, "数值", &["A"], &["共享", "不是整数"]);
    expect(&response, 1, "INVALID_ARGUMENT");
    assert!(response["message"].as_str().unwrap().contains("通道 1"));
}

#[test]
fn unexpected_adapter_panic_has_internal_code_and_releases_runtime_lock() {
    let runtime = Runtime::new();
    expect(
        &call(&runtime, Some(65_537), "框架异常", &[], &[]),
        3,
        "INTERNAL_ERROR",
    );
    assert!(!runtime.get().unwrap().is_poisoned());
    expect(
        &call(&runtime, Some(65_537), "GetFrameworkVersion", &[], &[]),
        0,
        "OK",
    );
}
