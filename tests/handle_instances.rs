#![cfg(feature = "instances")]
//! Handle 属于 Config 中的框架字段，不进入节点业务参数。

use miresidian::{
    Invocation, MiresidianError, MiresidianResult, MultiChannelInvocation, NodeRuntime,
    RuntimeInvocation, RuntimeResponse, ffi_runtime,
};
use std::{
    ffi::{CStr, CString},
    sync::{Mutex, OnceLock},
};

#[miresidian::node("Handle 回归", instances(
    mode = multiple, binding = handle, execution = selectable, default_execution = sequential
))]
struct HandleNode {
    name: String,
    speed: u32,
    calls: usize,
}

impl HandleNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("设备:device")] name: String,
        #[miresidian::config("速度:9600")] speed: u32,
    ) -> MiresidianResult<Self> {
        Ok(Self {
            name,
            speed,
            calls: 0,
        })
    }

    #[miresidian::operation("计数")]
    fn count(
        &mut self,
        #[miresidian::config("设备")] name: String,
        #[miresidian::config("速度")] speed: u32,
        #[miresidian::input("命令")] command: String,
    ) -> MiresidianResult<String> {
        assert_eq!(name, self.name);
        assert_eq!(speed, self.speed);
        self.calls += 1;
        Ok(format!(
            "{}:{}:{command}:{}",
            self.name, self.speed, self.calls
        ))
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}

fn request(operation: &str, config: &[&str], input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        input.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
    )
}

fn init(runtime: &mut NodeRuntime<HandleNode>, handle: &str, mode: &str) {
    runtime
        .invoke(&request(
            "InitDevice",
            &["device", "9600", handle, mode],
            &[],
        ))
        .unwrap();
}

#[test]
fn metadata_and_both_call_paths_agree_on_handle_position() {
    let mut runtime = NodeRuntime::<HandleNode>::new();
    assert_eq!(
        runtime
            .invoke(&request("GetInitConfig", &[], &[]))
            .unwrap()
            .as_slice(),
        [
            "设备:device",
            "速度:9600",
            "Handle",
            "平台适配:tb_lv_test_platform",
            "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播",
        ]
    );
    init(&mut runtime, "H1", "Sequential");
    // 调用只按 Handle 定位，节点 config 参数读取已保存的业务配置。
    assert_eq!(
        runtime
            .invoke(&request("计数", &["other", "1", "Handle:H1"], &["x"]))
            .unwrap()
            .as_slice(),
        ["device:9600:x:1"]
    );
    assert!(
        runtime
            .invoke(&request(
                "InitDevice",
                &["device", "9600", "H2", "Sequential", "extra"],
                &[]
            ))
            .is_err()
    );
}

#[test]
fn different_handles_keep_distinct_objects_even_with_identical_config() {
    let mut runtime = NodeRuntime::<HandleNode>::new();
    for handle in ["H1", "H2"] {
        init(&mut runtime, handle, "Sequential");
    }
    for handle in ["H1", "H2"] {
        assert_eq!(
            runtime
                .invoke(&request("计数", &["device", "9600", handle], &["x"]))
                .unwrap()
                .as_slice(),
            ["device:9600:x:1"]
        );
    }
    assert_eq!(
        runtime
            .invoke(&request("InitDevice", &["device", "9600", "H1"], &[]))
            .unwrap()
            .as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    assert!(
        matches!(runtime.invoke(&request("InitDevice", &["different", "9600", "H1"], &[])), Err(MiresidianError::InvalidArgument { field, .. }) if field == "Handle")
    );
    assert_eq!(
        runtime
            .invoke(&request("计数", &["device", "9600", "H1"], &["x"]))
            .unwrap()
            .as_slice(),
        ["device:9600:x:2"]
    );
}

#[test]
fn empty_or_missing_handle_skips_initialization_without_changing_objects() {
    let mut runtime = NodeRuntime::<HandleNode>::new();
    for config in [
        vec!["device", "9600"],
        vec!["device", "9600", ""],
        vec!["device", "9600", "  "],
        vec!["device", "9600", "Handle: "],
    ] {
        assert!(
            runtime
                .invoke(&request("InitDevice", &config, &[]))
                .unwrap()
                .as_slice()
                .is_empty()
        );
        assert!(!runtime.is_initialized());
    }
    init(&mut runtime, "live", "Sequential");
    // 跳过的通道不解析业务参数，也不影响已登记对象。
    assert!(
        runtime
            .invoke(&request(
                "InitDevice",
                &["unused", "invalid-speed", ""],
                &[]
            ))
            .unwrap()
            .as_slice()
            .is_empty()
    );
    assert_eq!(
        runtime
            .invoke(&request("计数", &["device", "9600", "live"], &["x"]))
            .unwrap()
            .as_slice(),
        ["device:9600:x:1"]
    );
}

#[test]
fn empty_unknown_and_closed_handles_never_fall_back_to_another_object() {
    let mut runtime = NodeRuntime::<HandleNode>::new();
    for handle in ["H1", "H2"] {
        init(&mut runtime, handle, "Sequential");
    }
    for operation in ["计数", "CloseDevice"] {
        assert!(
            runtime
                .invoke(&request(operation, &["device", "9600", ""], &["x"]))
                .is_err()
        );
        assert!(matches!(
            runtime.invoke(&request(operation, &["device", "9600", "missing"], &["x"])),
            Err(MiresidianError::UnknownChannel(_))
        ));
    }
    runtime
        .invoke(&request("CloseDevice", &["device", "9600", "H1"], &[]))
        .unwrap();
    assert!(matches!(
        runtime.invoke(&request("计数", &["device", "9600", "H1"], &["x"])),
        Err(MiresidianError::UnknownChannel(_))
    ));
    assert_eq!(
        runtime
            .invoke(&request("计数", &["device", "9600", "H2"], &["x"]))
            .unwrap()
            .as_slice(),
        ["device:9600:x:1"]
    );
}

#[test]
fn selectable_execution_belongs_to_each_handle_object() {
    let mut runtime = NodeRuntime::<HandleNode>::new();
    init(&mut runtime, "broadcast", "Broadcast");
    init(&mut runtime, "parallel", "Parallel");
    assert!(
        runtime
            .invoke(&request(
                "计数",
                &["device", "9600", "broadcast"],
                &["共享", "x", "y"]
            ))
            .is_err()
    );
    assert_eq!(
        runtime
            .invoke(&request(
                "计数",
                &["device", "9600", "broadcast"],
                &["共享", "x", "x"]
            ))
            .unwrap()
            .as_slice(),
        ["device:9600:x:1", "device:9600:x:2"]
    );
    let output = runtime
        .invoke(&request(
            "计数",
            &["device", "9600", "parallel"],
            &["共享", "x", "y"],
        ))
        .unwrap()
        .into_values();
    assert!(output[0].starts_with("device:9600:x:"));
    assert!(output[1].starts_with("device:9600:y:"));
    assert!(
        runtime
            .invoke(&request(
                "InitDevice",
                &["device", "9600", "parallel", "Sequential"],
                &[]
            ))
            .is_err()
    );
}

#[test]
fn structured_multi_channel_request_uses_the_handle_and_preserves_skips() {
    let mut runtime = NodeRuntime::<HandleNode>::new();
    init(&mut runtime, "unused", "Sequential");
    init(&mut runtime, "target", "Sequential");
    let response = runtime
        .invoke_command(RuntimeInvocation::MultiChannel(
            MultiChannelInvocation::new(
                "计数",
                vec!["device".to_owned(), "9600".to_owned(), "target".to_owned()],
                vec![Some(vec!["x".to_owned()]), None, Some(vec!["y".to_owned()])],
            ),
        ))
        .unwrap();
    let RuntimeResponse::MultiChannel(response) = response else {
        panic!("应返回多通道结果");
    };
    assert_eq!(response.slots.len(), 3);
    assert_eq!(
        response.slots[0].as_ref().unwrap().as_slice(),
        ["device:9600:x:1"]
    );
    assert!(response.slots[1].is_none());
    assert_eq!(
        response.slots[2].as_ref().unwrap().as_slice(),
        ["device:9600:y:2"]
    );
    assert_eq!(
        runtime
            .invoke(&request("计数", &["device", "9600", "unused"], &["x"]))
            .unwrap()
            .as_slice(),
        ["device:9600:x:1"]
    );
}

fn ffi_call(
    runtime: &OnceLock<Mutex<NodeRuntime<HandleNode>>>,
    explicit: bool,
    operation: &str,
    config: &[&str],
    input: &[&str],
) -> serde_json::Value {
    let operation = CString::new(operation).unwrap();
    let config = CString::new(serde_json::to_string(config).unwrap()).unwrap();
    let input = CString::new(serde_json::to_string(input).unwrap()).unwrap();
    // SAFETY: 输入字符串存活至调用结束，响应在下次调用前立即复制。
    let value = unsafe {
        let ptr = if explicit {
            ffi_runtime::invoke_with(
                runtime,
                1,
                operation.as_ptr(),
                config.as_ptr(),
                input.as_ptr(),
            )
        } else {
            ffi_runtime::invoke(runtime, operation.as_ptr(), config.as_ptr(), input.as_ptr())
        };
        CStr::from_ptr(ptr).to_str().unwrap()
    };
    serde_json::from_str(value).unwrap()
}

#[test]
fn both_ffi_entries_extract_handle_from_config_and_skip_blank_initialization() {
    for explicit in [false, true] {
        let runtime = OnceLock::new();
        let response = ffi_call(
            &runtime,
            explicit,
            "InitDevice",
            &["device", "9600", ""],
            &[],
        );
        assert_eq!(response["code"], "OK");
        assert_eq!(response["output"], serde_json::json!([]));
        assert!(!runtime.get().unwrap().lock().unwrap().is_initialized());
        for handle in ["H1", "H2"] {
            assert_eq!(
                ffi_call(
                    &runtime,
                    explicit,
                    "InitDevice",
                    &["device", "9600", handle],
                    &[]
                )["code"],
                "OK"
            );
            assert_eq!(
                ffi_call(
                    &runtime,
                    explicit,
                    "计数",
                    &["device", "9600", handle],
                    &["共享", "x", "x"]
                )["output"],
                serde_json::json!(["device:9600:x:1", "device:9600:x:2"])
            );
        }
        assert_eq!(
            ffi_call(&runtime, explicit, "CloseAllDevices", &[], &[])["code"],
            "OK"
        );
    }
}
