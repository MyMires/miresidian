//! 完整请求入口的跨语言契约与新旧入口共享状态回归。
use miresidian::{
    Invocation, InvocationOutput, MiresidianResult, NodeRuntime, Platform, PlatformAdapter,
    RuntimeInvocation, RuntimeResponse, ffi_runtime,
};
use serde_json::{Value, json};
use std::{
    ffi::{CStr, CString, c_char},
    sync::OnceLock,
};

#[derive(Default)]
struct CustomAdapter;
impl PlatformAdapter for CustomAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(65_536)
    }
    fn identifier(&self) -> &'static str {
        "request_test"
    }
    fn decode(&self, invocation: Invocation) -> MiresidianResult<RuntimeInvocation> {
        assert_ne!(invocation.operation, "AdapterPanic", "模拟适配器异常");
        Ok(RuntimeInvocation::Single(invocation))
    }
    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(output) => {
                let mut values = output.into_values();
                values.push("custom".to_owned());
                Ok(InvocationOutput::new(values))
            }
            RuntimeResponse::MultiChannel(_) => unreachable!("仅使用单通道调用"),
        }
    }
}

#[miresidian::node("请求入口验证", platforms(CustomAdapter))]
struct RequestNode {
    calls: u32,
}
impl RequestNode {
    #[miresidian::operation]
    fn init_device(address: String) -> MiresidianResult<Self> {
        let _ = address;
        Ok(Self { calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation]
    fn record(&mut self, text: String) -> MiresidianResult<(String, u32)> {
        self.calls += 1;
        Ok((text, self.calls))
    }
    #[miresidian::operation]
    fn crash(&mut self) -> MiresidianResult<()> {
        panic!("模拟节点异常")
    }
}

type Runtime = OnceLock<NodeRuntime<RequestNode>>;

unsafe fn copy_response(pointer: *const c_char) -> Value {
    // SAFETY: 调用者保证指针是当前线程刚返回的 ABI 响应，在下一次调用前读取并拥有结果。
    let raw = unsafe { CStr::from_ptr(pointer) }.to_str().unwrap();
    let response: Value = serde_json::from_str(raw).unwrap();
    assert_eq!(response.as_object().unwrap().len(), 4);
    assert!(raw.starts_with(&format!(
        "{{\"status\":{},\"code\":{},\"message\":",
        response["status"], response["code"]
    )));
    assert!(response["message"].is_string());
    assert!(
        response["output"]
            .as_array()
            .unwrap()
            .iter()
            .all(Value::is_string)
    );
    if response["status"] != 0 {
        assert_eq!(response["output"], json!([]));
        assert!(!response["message"].as_str().unwrap().is_empty());
    }
    response
}

fn request(runtime: &Runtime, text: &str) -> Value {
    let text = CString::new(text).unwrap();
    // SAFETY: 输入缓冲区在调用期间有效，返回响应立即复制。
    unsafe { copy_response(ffi_runtime::invoke(runtime, text.as_ptr())) }
}

fn call(
    runtime: &Runtime,
    platform: Option<u32>,
    function: &str,
    config: Value,
    input: Value,
) -> Value {
    let mut value = json!({"function":function,"config":config,"input":input});
    if let Some(platform) = platform {
        value["platform"] = json!(platform);
    }
    request(runtime, &value.to_string())
}

#[test]
fn absent_platform_and_numeric_platform_preserve_existing_routes() {
    for platform in [None, Some(0), Some(1), Some(65_536)] {
        let runtime = Runtime::new();
        let mut query = json!({"function":"GetInitConfig"});
        if let Some(platform) = platform {
            query["platform"] = json!(platform);
        }
        assert_eq!(
            request(&runtime, &query.to_string()),
            call(&runtime, platform, "GetInitConfig", json!([]), json!([]))
        );
        let config = json!(["A"]);
        let mut init = json!({"function":"InitDevice", "config":config});
        if let Some(platform) = platform {
            init["platform"] = json!(platform);
        }
        assert_eq!(request(&runtime, &init.to_string())["code"], "OK");
        let echoed = call(
            &runtime,
            platform,
            "Record",
            json!(["A"]),
            json!(["中文,\"\n\0"]),
        );
        let custom = platform == Some(65_536);
        let expected = if custom {
            json!(["中文,\"\n\0", "1", "custom"])
        } else {
            json!(["中文,\"\n\0", "1"])
        };
        assert_eq!(echoed["output"], expected);
        let mut record = json!({"function":"Record", "config":["A"], "input":["二"]});
        if let Some(platform) = platform {
            record["platform"] = json!(platform);
        }
        assert_eq!(request(&runtime, &record.to_string())["output"][1], "2");
        assert_eq!(
            call(&runtime, platform, "CloseDevice", json!(["A"]), json!([]))["code"],
            "OK"
        );
    }
}

#[test]
fn zero_and_omitted_platform_use_default_without_config_selection() {
    for platform in [None, Some(0), Some(1)] {
        let runtime = Runtime::new();
        for operation in ["GetFunctions", "GetInitConfig"] {
            let metadata = call(&runtime, platform, operation, json!([]), json!([]));
            assert_eq!(metadata["code"], "OK");
            assert!(
                metadata["output"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|value| !value.as_str().unwrap().contains("平台适配:"))
            );
        }
        assert_eq!(
            call(
                &runtime,
                platform,
                "InitDevice",
                json!(["A", "平台适配:request_test"]),
                json!([])
            )["code"],
            "INVALID_ARGUMENT"
        );
        assert_eq!(
            call(&runtime, platform, "InitDevice", json!(["A"]), json!([]))["code"],
            "OK"
        );
        assert_eq!(
            call(
                &runtime,
                platform,
                "Record",
                json!(["A"]),
                json!(["default"])
            )["output"],
            json!(["default", "1"])
        );
        assert_eq!(
            call(
                &runtime,
                Some(65536),
                "Record",
                json!(["A"]),
                json!(["wrong"])
            )["code"],
            "INVALID_ARGUMENT"
        );
        assert_eq!(
            call(&runtime, platform, "CloseDevice", json!(["A"]), json!([]))["code"],
            "OK"
        );
        assert_eq!(
            call(&runtime, Some(65536), "InitDevice", json!(["B"]), json!([]))["code"],
            "OK"
        );
        assert_eq!(
            call(
                &runtime,
                Some(65536),
                "Record",
                json!(["B"]),
                json!(["explicit"])
            )["output"],
            json!(["explicit", "1", "custom"])
        );
    }
}

#[test]
fn invalid_requests_are_rejected_before_runtime_creation() {
    let runtime = Runtime::new();
    for text in [
        "",
        "{",
        "null",
        "[]",
        r#"["GetFunctions",[],[],1]"#,
        "1",
        "true",
        r#"{}"#,
        r#"{"function":""}"#,
        r#"{"function":null}"#,
        r#"{"function":1}"#,
        r#"{"function":"GetFunctions","config":null}"#,
        r#"{"function":"GetFunctions","input":null}"#,
        r#"{"function":"GetFunctions","config":"[]"}"#,
        r#"{"function":"GetFunctions","input":{}}"#,
        r#"{"function":"InitDevice","config":[1]}"#,
        r#"{"function":"GetFunctions","input":[true]}"#,
        r#"{"function":"GetFunctions","input":[null]}"#,
        r#"{"function":"GetFunctions","input":[{}]}"#,
        r#"{"function":"GetFunctions","platform":null}"#,
        r#"{"function":"GetFunctions","platform":"1"}"#,
        r#"{"function":"GetFunctions","platform":true}"#,
        r#"{"function":"GetFunctions","platform":-1}"#,
        r#"{"function":"GetFunctions","platform":-0}"#,
        r#"{"function":"GetFunctions","platform":1.0}"#,
        r#"{"function":"GetFunctions","platform":1e0}"#,
        r#"{"function":"GetFunctions","platform":4294967296}"#,
        r#"{"function":"GetFunctions","platfrom":1}"#,
        r#"{"function":"GetFunctions","function":"InitDevice"}"#,
        r#"{"function":"GetFunctions","config":[],"config":[]}"#,
        r#"{"function":"GetFunctions","input":[],"input":[]}"#,
        r#"{"function":"GetFunctions","platform":1,"platform":1}"#,
        r#"{"function":"GetFunctions"} {}"#,
    ] {
        let response = request(&runtime, text);
        assert_eq!(response["status"], 1, "{text}: {response}");
        assert_eq!(response["code"], "INVALID_ARGUMENT", "{text}: {response}");
        assert!(runtime.get().is_none(), "非法请求不可创建运行时: {text}");
    }
    let invalid_utf8 = c"\xff";
    for pointer in [std::ptr::null(), invalid_utf8.as_ptr()] {
        // SAFETY: 非空指针指向有效 NUL 结尾缓冲区，非法 UTF-8 是被测参数错误。
        let response = unsafe { copy_response(ffi_runtime::invoke(&runtime, pointer)) };
        assert_eq!(response["code"], "INVALID_ARGUMENT");
        assert_eq!(response["status"], 1);
    }
    assert!(runtime.get().is_none());
}

#[test]
fn platform_numbers_keep_stable_codes_and_explicit_selector_rules() {
    let runtime = Runtime::new();
    for (platform, code) in [
        (2, "INVALID_PLATFORM_ID"),
        (65_535, "INVALID_PLATFORM_ID"),
        (65_537, "UNKNOWN_PLATFORM_ADAPTER"),
        (u32::MAX, "UNKNOWN_PLATFORM_ADAPTER"),
    ] {
        let response = request(
            &runtime,
            &json!({"platform":platform,"function":"GetFunctions"}).to_string(),
        );
        assert_eq!(response["status"], 1);
        assert_eq!(response["code"], code);
    }
    let response = request(
        &runtime,
        r#"{"platform":1,"function":"InitDevice","config":["A","平台适配:tb_lv_test_platform"]}"#,
    );
    assert_eq!(response["code"], "INVALID_ARGUMENT");
    assert!(
        response["message"]
            .as_str()
            .unwrap()
            .contains("不应在 Config 中重复选择")
    );
    let valid = request(
        &runtime,
        " \n{\"input\":[],\"config\":[],\"function\":\"GetFrameworkVersion\",\"platform\":1}\t",
    );
    assert_eq!(valid["output"], json!([miresidian::MIRESIDIAN_VERSION]));
}

#[test]
fn request_panics_return_errors_and_allow_cleanup_and_later_queries() {
    let runtime = Runtime::new();
    assert_eq!(
        request(&runtime, r#"{"function":"InitDevice","config":["A"]}"#)["code"],
        "OK"
    );
    let response = request(&runtime, r#"{"function":"Crash","config":["A"]}"#);
    assert_eq!(response["status"], 3);
    assert_eq!(response["code"], "NODE_PANICKED");
    assert_eq!(
        request(&runtime, r#"{"function":"CloseDevice","config":["A"]}"#)["code"],
        "OK"
    );
    let response = request(&runtime, r#"{"platform":65536,"function":"AdapterPanic"}"#);
    assert_eq!(response["status"], 3);
    assert_eq!(response["code"], "INTERNAL_ERROR");
    assert_eq!(
        request(&runtime, r#"{"function":"GetFunctions"}"#)["code"],
        "OK"
    );
}

#[test]
fn sole_json_export_shares_state_and_preserves_owned_response_copies() {
    assert_eq!(miresidian_abi_version(), 10);
    // SAFETY: 全部请求为有效 NUL 结尾 C 字符串，响应在下一次调用前复制。
    unsafe {
        assert_eq!(
            copy_response(miresidian_invoke(
                c"{\"function\":\"InitDevice\",\"config\":[\"A\"]}".as_ptr()
            ))["code"],
            "OK"
        );
        let first = copy_response(miresidian_invoke(
            c"{\"platform\":0,\"function\":\"Record\",\"config\":[\"A\"],\"input\":[\"one\"]}"
                .as_ptr(),
        ));
        let second = copy_response(miresidian_invoke(
            c"{\"platform\":1,\"function\":\"Record\",\"config\":[\"A\"],\"input\":[\"two\"]}"
                .as_ptr(),
        ));
        assert_eq!(first["output"], json!(["one", "1"]));
        assert_eq!(second["output"], json!(["two", "2"]));
        assert_eq!(
            copy_response(miresidian_invoke(
                c"{\"function\":\"CloseAllDevices\"}".as_ptr()
            ))["code"],
            "OK"
        );
        assert_eq!(
            copy_response(miresidian_invoke(
                c"{\"function\":\"Record\",\"config\":[\"A\"],\"input\":[\"x\"]}".as_ptr()
            ))["code"],
            "NOT_INITIALIZED"
        );
    }
}
