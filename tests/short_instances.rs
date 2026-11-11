#![cfg(feature = "instances")]
//! 裸标记归一到已有单对象顺序多通道策略，保持替换和关闭契约。
use miresidian::{
    DeviceNode, ExecutionMode, ExecutionPolicy, InstanceMode, Invocation, MiresidianResult,
    MultiChannelInvocation, NodeRuntime, RuntimeInvocation, RuntimeResponse,
};

#[miresidian::node("裸实例", instances)]
struct Node {
    calls: usize,
}
impl Node {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("名称:demo")] _name: String) -> MiresidianResult<Self> {
        Ok(Self { calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation]
    fn count(&mut self, value: String) -> MiresidianResult<String> {
        self.calls += 1;
        Ok(format!("{}:{value}", self.calls))
    }
}
fn request(operation: &str, config: &[&str], input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        input.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )
}
#[test]
fn bare_instances_orders_channels_and_keeps_one_object() {
    let policy = Node::descriptor().instances;
    assert_eq!(policy.mode, InstanceMode::Single);
    assert_eq!(
        policy.execution,
        ExecutionPolicy::Fixed(ExecutionMode::Sequential)
    );
    let runtime = NodeRuntime::<Node>::new();
    assert_eq!(
        runtime
            .invoke(&request("GetInitConfig", &[], &[]))
            .unwrap()
            .as_slice(),
        ["名称:demo", "平台适配:tb_lv_test_platform"]
    );
    runtime.invoke(&request("InitDevice", &["A"], &[])).unwrap();
    let response = runtime
        .invoke_command(RuntimeInvocation::MultiChannel(
            MultiChannelInvocation::new(
                "Count",
                vec!["A".into()],
                vec![Some(vec!["x".into()]), None, Some(vec!["y".into()])],
            ),
        ))
        .unwrap();
    let RuntimeResponse::MultiChannel(response) = response else {
        panic!("必须返回多通道结果")
    };
    assert_eq!(response.slots[0].as_ref().unwrap().as_slice(), ["1:x"]);
    assert!(response.slots[1].is_none());
    assert_eq!(response.slots[2].as_ref().unwrap().as_slice(), ["2:y"]);
    runtime.invoke(&request("InitDevice", &["B"], &[])).unwrap();
    assert!(runtime.invoke(&request("Count", &["A"], &["x"])).is_err());
    assert_eq!(
        runtime
            .invoke(&request("Count", &["B"], &["z"]))
            .unwrap()
            .as_slice(),
        ["1:z"]
    );
    runtime.shutdown().unwrap();
}
