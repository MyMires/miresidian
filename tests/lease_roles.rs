#![cfg(feature = "lease")]
use miresidian::{Invocation, MiresidianResult, Node as _, NodeRuntime};

#[miresidian::node("组合资源", lease)]
struct Node;
impl Node {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("地址", lease(key))] address: String,
        #[miresidian::config("端口:", lease(key))] port: u16,
        #[miresidian::config("速率:")] rate: u32,
        #[miresidian::config("超时:", lease(ignore))] timeout: u64,
    ) -> MiresidianResult<Self> {
        let _ = (address, port, rate, timeout);
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
fn config(port: &str, rate: &str, timeout: &str) -> Vec<String> {
    vec![
        format!("lease-roles:{}", std::process::id()),
        port.into(),
        rate.into(),
        timeout.into(),
    ]
}
#[test]
fn independent_runtimes_share_composite_identity_and_reject_conflicting_configuration() {
    let a = NodeRuntime::<Node>::new();
    assert_eq!(
        Node::descriptor().lease_activation,
        miresidian::LeaseActivation::Required
    );
    assert_eq!(
        a.invoke(&Invocation::new("GetInitConfig", vec![], vec![]))
            .unwrap()
            .as_slice(),
        ["地址", "端口:", "速率:", "超时:",]
    );
    let b = NodeRuntime::<Node>::new();
    let other_port = NodeRuntime::<Node>::new();
    let invoke = |runtime: &NodeRuntime<Node>, config| {
        runtime.invoke(&Invocation::new("InitDevice", config, vec![]))
    };
    invoke(&a, config("8000", "9600", "100")).unwrap();
    // Same normalized full initialization can attach to the owner.
    invoke(&b, config("0x1f40", "09600", "100")).unwrap();
    let different_state = NodeRuntime::<Node>::new();
    assert!(
        invoke(&different_state, config("8000", "9600", "200"))
            .unwrap_err()
            .to_string()
            .contains("完整初始化参数")
    );
    invoke(&other_port, config("9000", "4800", "100")).unwrap();
    let conflicting = NodeRuntime::<Node>::new();
    assert!(invoke(&conflicting, config("8000", "4800", "100")).is_err());
    // Lease roles work without instances; ignored timeout never changes lease bytes.
    let first = Node::lease_request(&config("8000", "9600", "100"))
        .unwrap()
        .unwrap();
    let second = Node::lease_request(&config("0x1f40", "09600", "200"))
        .unwrap()
        .unwrap();
    assert_eq!(first.resource(), second.resource());
    assert_eq!(first.configuration(), second.configuration());
    a.shutdown().unwrap();
    b.shutdown().unwrap();
    other_port.shutdown().unwrap();
    invoke(&conflicting, config("8000", "4800", "100")).unwrap();
}
