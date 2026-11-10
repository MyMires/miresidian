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
    unsafe { copy_response(ffi_runtime::invoke_request(runtime, text.as_ptr())) }
}

fn legacy(
    runtime: &Runtime,
    platform: Option<u32>,
    function: &str,
    config: Value,
    input: Value,
) -> Value {
    let function = CString::new(function).unwrap();
    let config = CString::new(config.to_string()).unwrap();
    let input = CString::new(input.to_string()).unwrap();
    // SAFETY: 输入 C 字符串在整个调用期间存活，响应在下一次调用前复制。
    unsafe {
        copy_response(match platform {
            Some(platform) => ffi_runtime::invoke_with(
                runtime,
                platform,
                function.as_ptr(),
                config.as_ptr(),
                input.as_ptr(),
            ),
            None => {
                ffi_runtime::invoke(runtime, function.as_ptr(), config.as_ptr(), input.as_ptr())
            }
        })
    }
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
            legacy(&runtime, platform, "GetInitConfig", json!([]), json!([]))
        );
        let mut config = json!(["A"]);
        // 省略 platform 或传 0 时通过 Config 选择自定义平台，不能补成 TB 编号 1。
        if platform.is_none() || platform == Some(0) {
            config
                .as_array_mut()
                .unwrap()
                .push(json!("平台适配:request_test"));
        }
        let mut init = json!({"function":"InitDevice", "config":config});
        if let Some(platform) = platform {
            init["platform"] = json!(platform);
        }
        assert_eq!(request(&runtime, &init.to_string())["code"], "OK");
        let echoed = legacy(
            &runtime,
            platform,
            "Record",
            json!(["A"]),
            json!(["中文,\"\n\0"]),
        );
        let custom = platform != Some(1);
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
            legacy(&runtime, platform, "CloseDevice", json!(["A"]), json!([]))["code"],
            "OK"
        );
    }
}

#[test]
fn zero_routes_metadata_selection_and_lifecycle_through_config() {
    for json_entry in [false, true] {
        let runtime = Runtime::new();
        let call = |function: &str, config: Value, input: Value| {
            if json_entry {
                request(
                    &runtime,
                    &json!({"platform":0,"function":function,"config":config,"input":input})
                        .to_string(),
                )
            } else {
                legacy(
                    &runtime,
                    Some(miresidian::MIRESIDIAN_PLATFORM_RUNTIME_SELECT),
                    function,
                    config,
                    input,
                )
            }
        };
        for function in ["GetInitConfig", "GetFunctions"] {
            let metadata = call(function, json!([]), json!([]));
            assert_eq!(metadata["code"], "OK");
            assert_eq!(
                metadata,
                legacy(&runtime, None, function, json!([]), json!([]))
            );
            assert!(metadata["output"].as_array().unwrap().iter().any(|value| {
                value
                    .as_str()
                    .unwrap()
                    .contains("平台适配:request_test/tb_lv_test_platform")
            }));
            for platform in [1, 65_536] {
                let explicit = legacy(&runtime, Some(platform), function, json!([]), json!([]));
                assert_eq!(explicit["code"], "OK");
                assert!(
                    explicit["output"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|value| { !value.as_str().unwrap().contains("平台适配:") })
                );
            }
        }
        let invalid = call("InitDevice", json!(["A", "平台适配:missing"]), json!([]));
        assert_eq!(invalid["code"], "UNKNOWN_PLATFORM_ADAPTER");
        assert_eq!(invalid["status"], 1);
        assert_eq!(
            call(
                "InitDevice",
                json!(["A", "平台适配:request_test"]),
                json!([])
            )["code"],
            "OK"
        );
        assert_eq!(
            call("Record", json!(["A"]), json!(["zero"]))["output"],
            json!(["zero", "1", "custom"])
        );
        // 省略平台与非零显式入口访问同一个实例，业务 Config 无需重复平台选择器。
        for (index, platform) in [None, Some(65_536)].into_iter().enumerate() {
            let response = legacy(
                &runtime,
                platform,
                "Record",
                json!(["A"]),
                json!(["shared"]),
            );
            assert_eq!(
                response["output"],
                json!(["shared", (index + 2).to_string(), "custom"])
            );
        }
        let conflict = call(
            "InitDevice",
            json!(["B", "平台适配:tb_lv_test_platform"]),
            json!([]),
        );
        assert_eq!(conflict["code"], "INVALID_ARGUMENT");
        assert_eq!(conflict["status"], 1);
        assert_eq!(
            call("Record", json!(["A"]), json!(["retained"]))["output"],
            json!(["retained", "4", "custom"])
        );
        assert_eq!(call("CloseDevice", json!(["A"]), json!([]))["code"], "OK");
        assert_eq!(
            call(
                "InitDevice",
                json!(["B", "平台适配:tb_lv_test_platform"]),
                json!([])
            )["code"],
            "OK"
        );
        assert_eq!(
            call("Record", json!(["B"]), json!(["tb"]))["output"],
            json!(["tb", "1"])
        );
        assert_eq!(call("CloseDevice", json!(["B"]), json!([]))["code"], "OK");
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
        let response = unsafe { copy_response(ffi_runtime::invoke_request(&runtime, pointer)) };
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
fn macro_exports_share_state_across_all_three_entries() {
    assert_eq!(miresidian_abi_version(), 9);
    // SAFETY: 全部 C 字符串在调用期间有效；每次响应立即读取到拥有的 JSON 值中。
    unsafe {
        let initialized = copy_response(miresidian_invoke(
            c"InitDevice".as_ptr(),
            c"[\"A\"]".as_ptr(),
            c"[]".as_ptr(),
        ));
        assert_eq!(initialized["code"], "OK");
        let first = copy_response(miresidian_invoke_request(
            c"{\"platform\":0,\"function\":\"Record\",\"config\":[\"A\"],\"input\":[\"one\"]}"
                .as_ptr(),
        ));
        assert_eq!(first["output"], json!(["one", "1"]));
        let second = copy_response(miresidian_invoke_with(
            miresidian::MIRESIDIAN_PLATFORM_RUNTIME_SELECT,
            c"Record".as_ptr(),
            c"[\"A\"]".as_ptr(),
            c"[\"two\"]".as_ptr(),
        ));
        assert_eq!(second["output"], json!(["two", "2"]));
        assert_eq!(first["output"], json!(["one", "1"])); // 拥有的副本跨调用仍有效。
        assert_eq!(
            copy_response(miresidian_invoke_request(
                c"{\"function\":\"CloseAllDevices\"}".as_ptr()
            ))["code"],
            "OK"
        );
        assert_eq!(
            copy_response(miresidian_invoke(
                c"Record".as_ptr(),
                c"[\"A\"]".as_ptr(),
                c"[\"x\"]".as_ptr()
            ))["code"],
            "NOT_INITIALIZED"
        );
    }
}
