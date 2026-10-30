#![cfg(feature = "instances")]
//! 固定策略、平台选择默认值与单实例多通道执行的运行时契约。

use miresidian::{
    DeviceNode, ExecutionMode, ExecutionPolicy, InstanceBinding, InstanceMode, InstancePolicy,
    Invocation, InvocationOutput, LeaseActivation, MiresidianResult, NodeDescriptor, NodeRuntime,
    OperationDescriptor, ParameterDescriptor,
};

const ARGUMENTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "值",
    definition: "值",
    default_value: None,
}];
const CONFIG: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "地址",
    definition: "地址",
    default_value: None,
}];
const OPERATIONS: [OperationDescriptor; 1] = [OperationDescriptor {
    name: "计数",
    inputs: &ARGUMENTS,
    outputs: &ARGUMENTS,
    channel_inputs: &ARGUMENTS,
}];
const fn descriptor(execution: ExecutionPolicy) -> NodeDescriptor {
    NodeDescriptor {
        name: "执行策略",
        init_config: &CONFIG,
        init_inputs: &[],
        init_outputs: &[],
        port_probe_inputs: &[],
        operations: &OPERATIONS,
        lease_activation: LeaseActivation::Disabled,
        instances: InstancePolicy {
            mode: InstanceMode::Single,
            binding: InstanceBinding::Config,
            execution,
        },
    }
}
const DESCRIPTORS: [NodeDescriptor; 4] = [
    descriptor(ExecutionPolicy::Fixed(ExecutionMode::Sequential)),
    descriptor(ExecutionPolicy::Fixed(ExecutionMode::Parallel)),
    descriptor(ExecutionPolicy::Fixed(ExecutionMode::Broadcast)),
    descriptor(ExecutionPolicy::Selectable(ExecutionMode::Broadcast)),
];

struct PolicyNode<const POLICY: usize> {
    address: String,
    calls: usize,
}
impl<const POLICY: usize> DeviceNode for PolicyNode<POLICY> {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTORS[POLICY]
    }
    fn init_device(config: &[String], _: &[String]) -> MiresidianResult<(Self, InvocationOutput)> {
        Ok((
            Self {
                address: config[0].clone(),
                calls: 0,
            },
            InvocationOutput::empty(),
        ))
    }
    fn invoke(
        &mut self,
        _: &str,
        _: &[String],
        input: &[String],
    ) -> MiresidianResult<InvocationOutput> {
        self.calls += 1;
        Ok(InvocationOutput::new(vec![format!(
            "{}:{}:{}",
            self.address, input[0], self.calls
        )]))
    }
    fn close_device(&mut self, _: &[String], _: &[String]) -> MiresidianResult<()> {
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

fn fixed<const POLICY: usize>() {
    let runtime = NodeRuntime::<PolicyNode<POLICY>>::new();
    assert_eq!(
        runtime
            .invoke(&request("GetInitConfig", &[], &[]))
            .unwrap()
            .as_slice(),
        ["地址", "平台适配:tb_lv_test_platform"]
    );
    assert!(
        runtime
            .invoke(&request("InitDevice", &["A", "Parallel"], &[]))
            .is_err()
    );
    runtime.invoke(&request("InitDevice", &["A"], &[])).unwrap();
    let mut output = runtime
        .invoke(&request("计数", &["A"], &["共享", "x", "x", "x"]))
        .unwrap()
        .into_values();
    if POLICY == 1 {
        output.sort();
    }
    if POLICY == 2 {
        assert_eq!(output, ["A:x:1", "A:x:1", "A:x:1"]);
    } else {
        assert_eq!(output, ["A:x:1", "A:x:2", "A:x:3"]);
    }
    runtime.invoke(&request("InitDevice", &["B"], &[])).unwrap();
    assert!(runtime.invoke(&request("计数", &["A"], &["x"])).is_err());
    assert_eq!(
        runtime
            .invoke(&request("计数", &["B"], &["x"]))
            .unwrap()
            .as_slice(),
        ["B:x:1"]
    );
}

#[test]
fn fixed_sequential_executes_inputs_in_order_on_one_instance() {
    fixed::<0>();
}

#[test]
fn fixed_parallel_serializes_object_mutation_and_preserves_output_positions() {
    fixed::<1>();
}

#[test]
fn fixed_broadcast_executes_channel_one_once_and_copies_its_output() {
    fixed::<2>();
    let runtime = NodeRuntime::<PolicyNode<2>>::new();
    runtime.invoke(&request("InitDevice", &["A"], &[])).unwrap();
    assert_eq!(
        runtime
            .invoke(&request("计数", &["A"], &["共享", "x", "y"]))
            .unwrap()
            .as_slice(),
        ["A:x:1", "A:x:1"]
    );
    assert_eq!(
        runtime
            .invoke(&request("计数", &["A"], &["after"]))
            .unwrap()
            .as_slice(),
        ["A:after:2"]
    );
}

#[test]
fn selectable_uses_the_declared_default_for_missing_or_blank_values() {
    for config in [
        vec!["A"],
        vec!["A", ""],
        vec!["A", "tb_lv_test_platform", " "],
    ] {
        let runtime = NodeRuntime::<PolicyNode<3>>::new();
        runtime
            .invoke(&request("InitDevice", &config, &[]))
            .unwrap();
        assert_eq!(
            runtime
                .invoke(&request("计数", &["A"], &["共享", "x", "y"]))
                .unwrap()
                .as_slice(),
            ["A:x:1", "A:x:1"]
        );
    }
    let runtime = NodeRuntime::<PolicyNode<3>>::new();
    runtime
        .invoke(&request("InitDevice", &["A", "Sequential"], &[]))
        .unwrap();
    assert_eq!(
        runtime
            .invoke(&request("计数", &["A"], &["共享", "x", "y"]))
            .unwrap()
            .as_slice(),
        ["A:x:1", "A:y:2"]
    );
}
