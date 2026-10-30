#![cfg(feature = "lease")]
//! 构造及初始化输出失败时，资源回收必须早于租约释放。
use miresidian::lease::{LeaseError, LeaseParameter, LeaseRequest, local_ipc::LocalLease};
use miresidian::{
    DeviceOutput, Invocation, InvocationOutput, MiresidianError, MiresidianResult, NodeRuntime,
};
use std::sync::Mutex;

static EVENTS: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

fn incompatible(resource: &str) -> Result<LocalLease, LeaseError> {
    let request = LeaseRequest::from_parameters(
        vec![
            LeaseParameter::new("resource", resource),
            LeaseParameter::new("setting", "2"),
        ],
        "resource",
        &["setting"],
    )
    .unwrap();
    LocalLease::acquire(request.resource().clone(), request.configuration().clone())
}
struct Resource(String);
impl Drop for Resource {
    fn drop(&mut self) {
        let protected = matches!(
            incompatible(&self.0),
            Err(LeaseError::IncompatibleConfiguration)
        );
        EVENTS.lock().unwrap().push((self.0.clone(), protected));
    }
}
struct Status(String);
impl DeviceOutput for Status {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        if self.0 == "encoding" {
            return Err(MiresidianError::Failure("encoding failed".into()));
        }
        Ok(InvocationOutput::empty())
    }
}
#[miresidian::node("构造失败租约", lease(activation = required, resource = "resource", configuration = ["setting"]))]
struct LeasedNode {
    resource: Resource,
}
impl LeasedNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("资源")] resource: String,
        #[miresidian::config("配置")] setting: u32,
        #[miresidian::input("模式")] mode: String,
    ) -> MiresidianResult<(Self, Status)> {
        assert_eq!(setting, 1);
        let resource = Resource(resource);
        if mode == "construct" {
            return Err(MiresidianError::Failure("construction failed".into()));
        }
        assert_ne!(mode, "panic", "construction panic");
        Ok((Self { resource }, Status(mode)))
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        // Observe the resource to ensure the node owns it until shutdown.
        assert!(!self.resource.0.is_empty());
        Ok(())
    }
}
#[test]
fn lease_is_held_during_resource_cleanup_and_released_after_each_failure() {
    for mode in ["construct", "panic", "encoding"] {
        let resource = format!("lease-construction:{}:{mode}", std::process::id());
        let runtime = NodeRuntime::<LeasedNode>::new();
        assert!(
            runtime
                .invoke(&Invocation::new(
                    "InitDevice",
                    vec![resource.clone(), "1".into()],
                    vec![mode.into()]
                ))
                .is_err()
        );
        assert!(!runtime.is_initialized());
        assert!(EVENTS.lock().unwrap().contains(&(resource.clone(), true)));
        incompatible(&resource)
            .expect("failed initialization must release its lease after cleanup")
            .release()
            .unwrap();
    }
}
