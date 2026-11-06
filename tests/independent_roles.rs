#![cfg(all(feature = "lease", feature = "instances"))]
//! 资源键仍独立于实例键；自动节点共享拒绝不同初始化状态。
use miresidian::{Invocation, MiresidianResult, NodeRuntime};
#[miresidian::node("独立角色", instances(mode = multiple, binding = config, execution = sequential), lease(activation = required))]
struct Node {
    address: u8,
    calls: u32,
}
impl Node {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("串口", instance(key), lease(key))] port_name: String,
        #[miresidian::config("设备地址:", instance(key), lease(ignore))] address: u8,
        #[miresidian::config("波特率:")] baud_rate: u32,
    ) -> MiresidianResult<Self> {
        let _ = (port_name, baud_rate);
        Ok(Self { address, calls: 0 })
    }
    #[miresidian::operation]
    fn read(&mut self) -> MiresidianResult<(u8, u32)> {
        self.calls += 1;
        Ok((self.address, self.calls))
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
#[test]
fn instance_selection_never_changes_resource_identity_or_compatibility_fields() {
    let runtime = NodeRuntime::<Node>::new();
    let port = format!("independent-roles:{}", std::process::id());
    let request = |op, address: &str, baud: &str| {
        Invocation::new(op, vec![port.clone(), address.into(), baud.into()], vec![])
    };
    runtime.invoke(&request("InitDevice", "1", "9600")).unwrap();
    let incompatible_state = runtime
        .invoke(&request("InitDevice", "2", "9600"))
        .unwrap_err();
    assert!(incompatible_state.to_string().contains("完整初始化参数"));
    // The same physical key still rejects incompatible resource configuration first.
    let error = runtime
        .invoke(&request("InitDevice", "3", "4800"))
        .unwrap_err();
    assert!(error.to_string().contains("不兼容配置"), "{error}");
    assert_eq!(
        runtime
            .invoke(&request("Read", "1", "unused"))
            .unwrap()
            .as_slice(),
        ["1", "1"]
    );
    runtime.shutdown().unwrap();
}
