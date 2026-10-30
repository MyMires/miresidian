#![cfg(feature = "instances")]
//! User acceptance: input [10, 20, empty, 30] calls channel 1 exactly once,
//! yielding [R, R, empty, R], for sync, native async and both C ABI entries.
use miresidian::{
    Invocation, InvocationOutput, MiresidianError, MiresidianResult, MultiChannelInvocation,
    NodeRuntime, RuntimeInvocation, RuntimeResponse, TaskService, TaskState,
};
use std::ffi::{CStr, CString};

#[miresidian::node("通道1广播验收",instances(mode=multiple,binding=config,execution=broadcast))]
struct BroadcastNode {
    calls: u32,
}
impl BroadcastNode {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("对象")] _address: String) -> MiresidianResult<Self> {
        Ok(Self { calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    fn evaluate(&mut self, value: u32) -> MiresidianResult<(u32, u32)> {
        self.calls += 1;
        if value == 99 {
            return Err(MiresidianError::Failure("source failed".into()));
        }
        Ok((self.calls, value))
    }
    #[miresidian::operation]
    fn read(&mut self, value: u32) -> MiresidianResult<(u32, u32)> {
        self.evaluate(value)
    }
    #[miresidian::operation]
    async fn read_async(&mut self, value: u32) -> MiresidianResult<(u32, u32)> {
        tokio::task::yield_now().await;
        self.evaluate(value)
    }
    #[miresidian::operation]
    fn touch(&mut self) -> MiresidianResult<()> {
        self.calls += 1;
        Ok(())
    }
    #[miresidian::operation]
    fn calls(&mut self) -> MiresidianResult<u32> {
        Ok(self.calls)
    }
}
fn invocation(operation: &str, key: &str, input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        vec![key.into()],
        input.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
    )
}
fn runtime() -> NodeRuntime<BroadcastNode> {
    let runtime = NodeRuntime::new();
    runtime.invoke(&invocation("InitDevice", "A", &[])).unwrap();
    runtime
}
fn slots(values: &[Option<&str>]) -> Vec<Option<Vec<String>>> {
    values
        .iter()
        .map(|value| value.map(|v| vec![v.into()]))
        .collect()
}
fn broadcast(
    runtime: &NodeRuntime<BroadcastNode>,
    operation: &str,
    slots: Vec<Option<Vec<String>>>,
) -> MiresidianResult<Vec<Option<InvocationOutput>>> {
    match runtime.invoke_command(RuntimeInvocation::MultiChannel(
        MultiChannelInvocation::new(operation, vec!["A".into()], slots),
    ))? {
        RuntimeResponse::MultiChannel(response) => Ok(response.slots),
        _ => panic!("expected batch"),
    }
}
fn calls(runtime: &NodeRuntime<BroadcastNode>) -> u32 {
    runtime
        .invoke(&invocation("Calls", "A", &[]))
        .unwrap()
        .as_slice()[0]
        .parse()
        .unwrap()
}

#[test]
fn sync_and_native_async_execute_channel_one_once_and_clone_all_output_fields() {
    for operation in ["Read", "ReadAsync"] {
        let runtime = runtime();
        let result = broadcast(
            &runtime,
            operation,
            slots(&[
                Some("10"),
                Some("20"),
                None,
                Some("not-a-number"),
                Some(""),
                Some("30"),
            ]),
        )
        .unwrap();
        let output = Some(InvocationOutput::new(vec!["1".into(), "10".into()]));
        assert_eq!(
            result,
            vec![
                output.clone(),
                output.clone(),
                None,
                output.clone(),
                None,
                output
            ]
        );
        assert_eq!(
            calls(&runtime),
            1,
            "a broadcast must execute one business method"
        );
    }
}
#[test]
fn missing_or_structurally_invalid_source_never_falls_back_to_another_channel() {
    for operation in ["Read", "ReadAsync"] {
        for first in [
            None,
            Some(vec![]),
            Some(vec!["".into()]),
            Some(vec!["10".into(), "extra".into()]),
        ] {
            let runtime = runtime();
            let result =
                broadcast(&runtime, operation, vec![first, Some(vec!["20".into()])]).unwrap();
            assert_eq!(result, vec![None, None]);
            assert_eq!(calls(&runtime), 0);
        }
        let runtime = runtime();
        assert!(broadcast(&runtime, operation, vec![]).unwrap().is_empty());
        assert_eq!(calls(&runtime), 0);
    }
}
#[test]
fn invalid_source_value_returns_its_error_and_does_not_execute_other_inputs() {
    for operation in ["Read", "ReadAsync"] {
        let runtime = runtime();
        let error = broadcast(
            &runtime,
            operation,
            slots(&[Some("bad-number"), Some("10")]),
        )
        .unwrap_err();
        assert_eq!(error.code(), "INVALID_ARGUMENT");
        assert!(error.to_string().contains("通道 1"));
        assert_eq!(calls(&runtime), 0);
    }
}
#[test]
fn source_business_failure_is_not_retried_on_other_channels() {
    for operation in ["Read", "ReadAsync"] {
        let runtime = runtime();
        let error = broadcast(
            &runtime,
            operation,
            slots(&[Some("99"), Some("10"), Some("20")]),
        )
        .unwrap_err();
        assert!(error.to_string().contains("通道 1"));
        assert_eq!(calls(&runtime), 1);
    }
}
#[test]
fn zero_argument_void_output_broadcast_keeps_valid_and_skipped_slots_distinct() {
    let runtime = runtime();
    let result = broadcast(&runtime, "Touch", vec![Some(vec![]), None, Some(vec![])]).unwrap();
    assert_eq!(
        result,
        vec![
            Some(InvocationOutput::empty()),
            None,
            Some(InvocationOutput::empty())
        ]
    );
    assert_eq!(calls(&runtime), 1);
}
#[test]
fn task_service_uses_the_same_channel_one_broadcast_contract() {
    let runtime = runtime();
    let service = TaskService::new(runtime.clone(), 1).unwrap();
    let id = service
        .submit(
            invocation("ReadAsync", "A", &["共享", "10", "20", "", "30"]),
            None,
            None,
        )
        .unwrap();
    let output = service.wait(id, None).unwrap();
    assert_eq!(output.state, TaskState::Succeeded);
    assert_eq!(output.output, ["1,10", "1,10", "", "1,10"]);
    assert_eq!(calls(&runtime), 1);
    service.shutdown().unwrap();
    service.release(id).unwrap();
}
fn ffi(explicit: bool, operation: &str, key: &str, input: &[&str]) -> serde_json::Value {
    let operation = CString::new(operation).unwrap();
    let config = CString::new(serde_json::to_string(&[key]).unwrap()).unwrap();
    let input = CString::new(serde_json::to_string(input).unwrap()).unwrap();
    // SAFETY: all inputs are live C strings; response is copied on the same
    // thread before another framework call replaces its response cache.
    let text = unsafe {
        let pointer = if explicit {
            miresidian_invoke_with(1, operation.as_ptr(), config.as_ptr(), input.as_ptr())
        } else {
            miresidian_invoke(operation.as_ptr(), config.as_ptr(), input.as_ptr())
        };
        CStr::from_ptr(pointer).to_str().unwrap().to_owned()
    };
    serde_json::from_str(&text).unwrap()
}
#[test]
fn both_existing_c_entries_broadcast_the_first_channels_single_result() {
    for explicit in [false, true] {
        for operation in ["Read", "ReadAsync"] {
            let key = format!("ffi-{explicit}-{operation}");
            assert_eq!(ffi(explicit, "InitDevice", &key, &[])["code"], "OK");
            let output = ffi(
                explicit,
                operation,
                &key,
                &["共享", "10", "20", "", "bad-number", "30"],
            );
            assert_eq!(output["code"], "OK");
            assert_eq!(
                output["output"],
                serde_json::json!(["1,10", "1,10", "", "1,10", "1,10"])
            );
            assert_eq!(
                ffi(explicit, "Calls", &key, &[])["output"],
                serde_json::json!(["1"])
            );
            let skipped = ffi(explicit, operation, &key, &["共享", "", "20"]);
            assert_eq!(skipped["code"], "OK");
            assert_eq!(skipped["output"], serde_json::json!(["", ""]));
            assert_eq!(
                ffi(explicit, "Calls", &key, &[])["output"],
                serde_json::json!(["1"])
            );
            assert_eq!(ffi(explicit, "CloseDevice", &key, &[])["code"], "OK");
        }
    }
}
