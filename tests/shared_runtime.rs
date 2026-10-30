#![cfg(feature = "instances")]
//! 平台多通道输入通过唯一 Config 共享同一有状态对象。

use miresidian::{
    Invocation, MiresidianError, MiresidianResult, MultiChannelInvocation, NodeRuntime,
    RuntimeInvocation, RuntimeResponse, TB_LV_TEST_PLATFORM, TbLvTestPlatformAdapter, ffi_runtime,
};
use std::{
    ffi::{CStr, CString},
    sync::{Mutex, OnceLock},
};

#[miresidian::node("共享对象回归", instances(mode = multiple, binding = config, execution = selectable, default_execution = sequential))]
struct SharedNode {
    address: String,
    calls: usize,
}

impl SharedNode {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("地址:A")] address: String) -> MiresidianResult<Self> {
        Ok(Self { address, calls: 0 })
    }

    #[miresidian::operation("记录")]
    fn record(
        &mut self, #[miresidian::input("命令")] command: String
    ) -> MiresidianResult<String> {
        self.calls += 1;
        Ok(format!("{}:{command}:{}", self.address, self.calls))
    }

    #[miresidian::operation("读取配置")]
    fn config(
        &mut self,
        #[miresidian::config("地址")] address: String,
    ) -> MiresidianResult<String> {
        Ok(format!("{}:{address}", self.address))
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

#[test]
fn shared_inputs_use_one_config_selected_stateful_object_in_all_modes() {
    for mode in ["Sequential", "Parallel", "Broadcast"] {
        let runtime = NodeRuntime::<SharedNode>::new();
        // 先创建无关对象，防止按创建顺序分发也能误通过测试。
        for address in ["B", "A"] {
            runtime
                .invoke(&request("InitDevice", &[address, mode], &[]))
                .unwrap();
        }
        for _ in 0..2 {
            assert_eq!(
                runtime
                    .invoke(&request("InitDevice", &["A", mode], &[]))
                    .unwrap()
                    .as_slice(),
                [miresidian::CHANNEL_REUSED_MESSAGE]
            );
        }
        let output = runtime
            .invoke(&request(
                "记录",
                &["A"],
                &["共享", "x", "", "x", "x", "x,excess"],
            ))
            .unwrap()
            .into_values();
        assert_eq!(output.len(), 5);
        assert_eq!(output[1], "");
        assert_eq!(output[4], "");
        let mut executed = [output[0].clone(), output[2].clone(), output[3].clone()];
        if mode == "Parallel" {
            executed.sort();
        }
        if mode == "Broadcast" {
            assert_eq!(executed, ["A:x:1", "A:x:1", "A:x:1"]);
        } else {
            assert_eq!(executed, ["A:x:1", "A:x:2", "A:x:3"]);
        }
        assert_eq!(
            runtime
                .invoke(&request("记录", &["A"], &["after"]))
                .unwrap()
                .as_slice(),
            [if mode == "Broadcast" {
                "A:after:2"
            } else {
                "A:after:4"
            }]
        );
        assert_eq!(
            runtime
                .invoke(&request("记录", &["B"], &["untouched"]))
                .unwrap()
                .as_slice(),
            ["B:untouched:1"]
        );
    }
}

#[test]
fn shared_unknown_or_closed_config_never_selects_another_object() {
    let runtime = NodeRuntime::<SharedNode>::new();
    for address in ["B", "A"] {
        runtime
            .invoke(&request("InitDevice", &[address], &[]))
            .unwrap();
    }
    assert!(matches!(
        runtime.invoke(&request("记录", &["missing"], &["共享", "x"])),
        Err(MiresidianError::UnknownChannel(key)) if key == "missing"
    ));
    runtime
        .invoke(&request("CloseDevice", &["A"], &[]))
        .unwrap();
    assert!(matches!(
        runtime.invoke_command(RuntimeInvocation::MultiChannel(MultiChannelInvocation::new(
            "记录",
            vec!["A".to_owned()],
            vec![Some(vec!["x".to_owned()])],
        ))),
        Err(MiresidianError::UnknownChannel(key)) if key == "A"
    ));
    assert_eq!(
        runtime
            .invoke(&request("记录", &["B"], &["x"]))
            .unwrap()
            .as_slice(),
        ["B:x:1"]
    );
}

#[test]
fn shared_config_uses_defaults_prefixes_and_reserved_config_rules() {
    let runtime = NodeRuntime::<SharedNode>::new();
    runtime.invoke(&request("InitDevice", &["A"], &[])).unwrap();
    for config in [
        vec![],
        vec!["地址:A"],
        vec!["A", TB_LV_TEST_PLATFORM, "Sequential"],
    ] {
        assert_eq!(
            runtime
                .invoke(&request("记录", &config, &["共享", "x"]))
                .unwrap()
                .as_slice()
                .len(),
            1
        );
    }
    assert!(matches!(
        runtime.invoke(&request(
            "记录",
            &["A", "Sequential", "excess"],
            &["共享", "x"]
        )),
        Err(MiresidianError::InvalidArgument { .. })
    ));
    let error = runtime
        .invoke_with(
            &TbLvTestPlatformAdapter,
            request("记录", &["A", TB_LV_TEST_PLATFORM], &["共享", "x"]),
        )
        .unwrap_err();
    assert!(error.to_string().contains("不应在 Config 中重复选择"));
    assert_eq!(
        runtime
            .invoke(&request("记录", &["A"], &["after"]))
            .unwrap()
            .as_slice(),
        ["A:after:4"]
    );
}

#[test]
fn structured_zero_input_calls_share_normalized_config_and_preserve_positions() {
    let runtime = NodeRuntime::<SharedNode>::new();
    runtime.invoke(&request("InitDevice", &["A"], &[])).unwrap();
    let response = runtime
        .invoke_command(RuntimeInvocation::MultiChannel(
            MultiChannelInvocation::new(
                "读取配置",
                vec!["地址:A".to_owned()],
                vec![Some(vec![]), None, Some(vec![])],
            ),
        ))
        .unwrap();
    let RuntimeResponse::MultiChannel(response) = response else {
        panic!("共享调用应保留结构化结果");
    };
    assert_eq!(response.slots.len(), 3);
    assert_eq!(response.slots[0].as_ref().unwrap().as_slice(), ["A:A"]);
    assert!(response.slots[1].is_none());
    assert_eq!(response.slots[2].as_ref().unwrap().as_slice(), ["A:A"]);
}

#[test]
fn shared_calls_before_initialization_are_rejected() {
    let runtime = NodeRuntime::<SharedNode>::new();
    assert!(matches!(
        runtime.invoke(&request("记录", &["A"], &["共享", "x", "x"])),
        Err(MiresidianError::NotInitialized)
    ));
}

type Runtime = OnceLock<Mutex<NodeRuntime<SharedNode>>>;

fn call(
    runtime: &Runtime,
    explicit: bool,
    operation: &str,
    config: &[&str],
    input: &[&str],
) -> serde_json::Value {
    let operation = CString::new(operation).unwrap();
    let config = CString::new(serde_json::to_string(config).unwrap()).unwrap();
    let input = CString::new(serde_json::to_string(input).unwrap()).unwrap();
    // SAFETY: 输入 CString 存活至调用结束，返回文本在下一次调用前立即复制并解析。
    let response = unsafe {
        let pointer = if explicit {
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
        CStr::from_ptr(pointer).to_str().unwrap()
    };
    serde_json::from_str(response).unwrap()
}

#[test]
fn both_ffi_entries_accept_multiple_platform_inputs_for_one_reused_object() {
    for explicit in [false, true] {
        for mode in ["Sequential", "Parallel", "Broadcast"] {
            let runtime = Runtime::new();
            assert_eq!(
                call(&runtime, explicit, "InitDevice", &["A", mode], &[])["code"],
                "OK"
            );
            assert_eq!(
                call(&runtime, explicit, "InitDevice", &["A", mode], &[])["output"],
                serde_json::json!([miresidian::CHANNEL_REUSED_MESSAGE])
            );
            let response = call(&runtime, explicit, "记录", &["A"], &["共享", "x", "x"]);
            assert_eq!(response["code"], "OK");
            let mut output: Vec<String> =
                serde_json::from_value(response["output"].clone()).unwrap();
            if mode == "Parallel" {
                output.sort();
            }
            if mode == "Broadcast" {
                assert_eq!(output, ["A:x:1", "A:x:1"]);
            } else {
                assert_eq!(output, ["A:x:1", "A:x:2"]);
            }
            assert_eq!(
                call(&runtime, explicit, "记录", &["missing"], &["共享", "x"])["code"],
                "UNKNOWN_CHANNEL"
            );
            assert_eq!(
                call(&runtime, explicit, "CloseDevice", &["A"], &[])["code"],
                "OK"
            );
        }
    }
}

#[test]
fn blank_config_primary_keys_skip_creation_before_defaults_are_applied() {
    let runtime = NodeRuntime::<SharedNode>::new();
    let blanks = [
        vec![],
        vec![""],
        vec!["  "],
        vec!["地址:"],
        vec!["", TB_LV_TEST_PLATFORM, "Parallel"],
    ];
    for config in &blanks {
        assert!(
            runtime
                .invoke(&request("InitDevice", config, &[]))
                .unwrap()
                .as_slice()
                .is_empty()
        );
        assert!(
            !runtime.is_initialized(),
            "框架字段和默认值不能将空业务主键变成新对象"
        );
    }
    runtime.invoke(&request("InitDevice", &["A"], &[])).unwrap();
    runtime.invoke(&request("记录", &["A"], &["x"])).unwrap();
    for config in &blanks {
        assert!(
            runtime
                .invoke(&request("InitDevice", config, &[]))
                .unwrap()
                .as_slice()
                .is_empty()
        );
    }
    assert_eq!(
        runtime
            .invoke(&request("记录", &["A"], &["x"]))
            .unwrap()
            .as_slice(),
        ["A:x:2"]
    );
}
