#![cfg(all(feature = "lease", feature = "instances"))]
#[path = "../examples/support/leased_node.rs"]
mod node;
use miresidian::{Invocation, MiresidianError, NodeRuntime};
use node::LeasedNode;
use std::{
    fs,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Resource(String);
impl Resource {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "miresidian-node-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, []).unwrap();
        Self(path.to_str().unwrap().into())
    }
    fn call(
        &self,
        runtime: &NodeRuntime<LeasedNode>,
        op: &str,
        handle: &str,
        setting: &str,
        mode: &str,
        input: &[&str],
    ) -> miresidian::MiresidianResult<miresidian::InvocationOutput> {
        runtime.invoke(&Invocation::new(
            op,
            vec![self.0.clone(), setting.into(), handle.into(), mode.into()],
            input.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
        ))
    }
    fn events(&self) -> Vec<String> {
        fs::read_to_string(format!("{}.events", self.0))
            .unwrap()
            .lines()
            .map(Into::into)
            .collect()
    }
}
impl Drop for Resource {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
        let _ = fs::remove_file(format!("{}.events", self.0));
    }
}

#[test]
fn lease_initializes_once_and_only_last_reference_closes_and_drops() {
    let resource = Resource::new();
    let a = NodeRuntime::new();
    let b = NodeRuntime::new();
    for runtime in [&a, &b] {
        assert_eq!(
            resource
                .call(runtime, "InitDevice", "h", "1", "Lease", &[])
                .unwrap()
                .as_slice(),
            ["connected:1"]
        );
    }
    resource
        .call(&a, "InitDevice", "h2", "1", "Lease", &[])
        .unwrap();
    assert_eq!(
        resource
            .call(&b, "查询", "h", "1", "Lease", &["first"])
            .unwrap()
            .as_slice(),
        ["1:first"]
    );
    resource
        .call(&a, "CloseDevice", "h", "1", "Lease", &[])
        .unwrap();
    a.shutdown().unwrap();
    assert!(!resource.events().iter().any(|s| s == "close"));
    assert_eq!(
        resource
            .call(&b, "异步查询", "h", "1", "Lease", &[])
            .unwrap()
            .as_slice(),
        ["2"]
    );
    let err = resource
        .call(&b, "参数错误", "h", "1", "Lease", &[])
        .unwrap_err();
    assert_eq!(err.code(), "INVALID_ARGUMENT");
    assert!(err.to_string().contains("远端"));
    b.shutdown().unwrap();
    let events = resource.events();
    assert_eq!(events.iter().filter(|e| e.starts_with("open:")).count(), 1);
    assert_eq!(&events[events.len() - 2..], ["close", "drop"]);
    resource
        .call(&a, "InitDevice", "new", "2", "Lease", &[])
        .unwrap();
    a.shutdown().unwrap();
    assert_eq!(
        resource
            .events()
            .iter()
            .filter(|e| e.starts_with("open:"))
            .count(),
        2
    );
}
#[test]
fn conflicting_lease_and_direct_second_open_are_rejected_without_harming_owner() {
    let r = Resource::new();
    let a = NodeRuntime::new();
    let b = NodeRuntime::new();
    r.call(&a, "InitDevice", "a", "1", "Lease", &[]).unwrap();
    assert!(
        r.call(&b, "InitDevice", "b", "2", "Lease", &[])
            .unwrap_err()
            .to_string()
            .contains("不兼容配置")
    );
    assert!(
        r.call(&b, "InitDevice", "b", "1", "Direct", &[])
            .unwrap_err()
            .to_string()
            .contains("重复打开")
    );
    assert_eq!(
        r.call(&a, "查询", "a", "1", "Lease", &["alive"])
            .unwrap()
            .as_slice(),
        ["1:alive"]
    );
    a.shutdown().unwrap();
}
#[test]
fn panic_fault_is_shared_and_last_close_allows_fresh_initialization() {
    let r = Resource::new();
    let a = NodeRuntime::new();
    let b = NodeRuntime::new();
    for rt in [&a, &b] {
        r.call(rt, "InitDevice", "a", "1", "Lease", &[]).unwrap();
    }
    assert!(matches!(
        r.call(&a, "异常", "a", "1", "Lease", &[]),
        Err(MiresidianError::NodePanicked { .. })
    ));
    assert!(matches!(
        r.call(&b, "查询", "a", "1", "Lease", &["blocked"]),
        Err(MiresidianError::NodePanicked { .. })
    ));
    a.shutdown().unwrap();
    b.shutdown().unwrap();
    r.call(&b, "InitDevice", "b", "1", "Lease", &[]).unwrap();
    b.shutdown().unwrap();
}

// 手写适配器仅用于制造不同的命名契约；复用原节点的资源键和配置编码。
struct OtherContract(LeasedNode);
impl miresidian::DeviceNode for OtherContract {
    fn descriptor() -> &'static miresidian::NodeDescriptor {
        static DESCRIPTOR: std::sync::OnceLock<miresidian::NodeDescriptor> =
            std::sync::OnceLock::new();
        DESCRIPTOR.get_or_init(|| miresidian::NodeDescriptor {
            name: "不兼容节点接口",
            ..*<LeasedNode as miresidian::DeviceNode>::descriptor()
        })
    }
    fn init_device(
        config: &[String],
        input: &[String],
    ) -> miresidian::MiresidianResult<(Self, miresidian::InvocationOutput)> {
        <LeasedNode as miresidian::DeviceNode>::init_device(config, input)
            .map(|(node, output)| (Self(node), output))
    }
    fn lease_request(
        config: &[String],
    ) -> miresidian::MiresidianResult<Option<miresidian::lease::LeaseRequest>> {
        <LeasedNode as miresidian::DeviceNode>::lease_request(config)
    }
    fn close_device(
        &mut self,
        config: &[String],
        input: &[String],
    ) -> miresidian::MiresidianResult<()> {
        <LeasedNode as miresidian::DeviceNode>::close_device(&mut self.0, config, input)
    }
    fn invoke(
        &mut self,
        operation: &str,
        config: &[String],
        input: &[String],
    ) -> miresidian::MiresidianResult<miresidian::InvocationOutput> {
        <LeasedNode as miresidian::DeviceNode>::invoke(&mut self.0, operation, config, input)
    }
}
#[test]
fn different_named_contract_is_rejected_before_opening_a_second_resource() {
    let resource = Resource::new();
    let first = NodeRuntime::new();
    resource
        .call(&first, "InitDevice", "h", "1", "Lease", &[])
        .unwrap();
    let other = NodeRuntime::<OtherContract>::new();
    let error = other
        .invoke(&Invocation::new(
            "InitDevice",
            vec![
                resource.0.clone(),
                "1".into(),
                "other".into(),
                "Lease".into(),
            ],
            vec![],
        ))
        .unwrap_err();
    assert!(matches!(error, MiresidianError::InvalidArgument { .. }));
    assert!(error.to_string().contains("节点契约不兼容"));
    assert!(!other.is_initialized());
    assert_eq!(
        resource
            .call(&first, "查询", "h", "1", "Lease", &["alive"])
            .unwrap()
            .as_slice(),
        ["1:alive"]
    );
    first.shutdown().unwrap();
}
