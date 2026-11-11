//! 默认单实例约束在所有 feature 组合下保持不变。

use miresidian::{Invocation, MiresidianError, MiresidianResult, Node, NodeRuntime};
use std::sync::Mutex;

static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[miresidian::node("默认单实例")]
struct Singleton {
    name: String,
    calls: usize,
}

impl Singleton {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("名称:default")] name: String,
        #[miresidian::config("速度:1")] speed: u32,
    ) -> MiresidianResult<Self> {
        assert!(speed > 0);
        EVENTS.lock().unwrap().push(format!("create:{name}"));
        if name == "construct-fail" {
            return Err(MiresidianError::Failure("构造失败".to_owned()));
        }
        Ok(Self { name, calls: 0 })
    }

    #[miresidian::operation("计数")]
    fn count(&mut self) -> MiresidianResult<String> {
        self.calls += 1;
        Ok(format!("{}:{}", self.name, self.calls))
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        EVENTS.lock().unwrap().push(format!("close:{}", self.name));
        if self.name == "close-fail" {
            return Err(MiresidianError::Failure("关闭失败".to_owned()));
        }
        Ok(())
    }
}

impl Drop for Singleton {
    fn drop(&mut self) {
        EVENTS.lock().unwrap().push(format!("drop:{}", self.name));
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
fn default_metadata_never_publishes_handle_or_execution_selector() {
    assert_eq!(
        Singleton::descriptor().instance,
        miresidian::InstancePolicy::DEFAULT
    );
    let runtime = NodeRuntime::<Singleton>::new();
    assert_eq!(
        runtime
            .invoke(&request("GetInitConfig", &[], &[]))
            .unwrap()
            .as_slice(),
        ["名称:default", "速度:1"]
    );
    assert!(
        runtime
            .invoke(&request("InitDevice", &["x", "1", "Parallel"], &[]))
            .is_err()
    );
    assert!(!runtime.is_initialized());
}

#[test]
fn different_config_replaces_the_only_object_after_close_and_drop() {
    let runtime = NodeRuntime::<Singleton>::new();
    runtime
        .invoke(&request("InitDevice", &["replace-A", "1"], &[]))
        .unwrap();
    runtime
        .invoke(&request("计数", &["replace-A", "1"], &[]))
        .unwrap();
    assert_eq!(
        runtime
            .invoke(&request("InitDevice", &["replace-A", "1"], &[]))
            .unwrap()
            .as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    runtime
        .invoke(&request("InitDevice", &["replace-B", "1"], &[]))
        .unwrap();
    assert!(matches!(
        runtime.invoke(&request("计数", &["replace-A", "1"], &[])),
        Err(MiresidianError::UnknownChannel(_))
    ));
    assert_eq!(
        runtime
            .invoke(&request("计数", &["replace-B", "1"], &[]))
            .unwrap()
            .as_slice(),
        ["replace-B:1"]
    );
    let events = EVENTS.lock().unwrap();
    let position = |name| events.iter().position(|event| event == name).unwrap();
    assert!(position("close:replace-A") < position("drop:replace-A"));
    assert!(position("drop:replace-A") < position("create:replace-B"));
    drop(events);
    runtime.shutdown().unwrap();
}

#[test]
fn typed_prevalidation_failure_keeps_the_old_object() {
    let runtime = NodeRuntime::<Singleton>::new();
    runtime
        .invoke(&request("InitDevice", &["validated-A", "1"], &[]))
        .unwrap();
    assert!(
        runtime
            .invoke(&request(
                "InitDevice",
                &["validated-B", "not-a-number"],
                &[]
            ))
            .is_err()
    );
    assert_eq!(
        runtime
            .invoke(&request("计数", &["validated-A", "1"], &[]))
            .unwrap()
            .as_slice(),
        ["validated-A:1"]
    );
    assert!(
        !EVENTS
            .lock()
            .unwrap()
            .contains(&"create:validated-B".to_owned())
    );
}

#[test]
fn construction_failure_after_replacement_leaves_no_object() {
    let runtime = NodeRuntime::<Singleton>::new();
    runtime
        .invoke(&request("InitDevice", &["before-failure", "1"], &[]))
        .unwrap();
    assert!(
        runtime
            .invoke(&request("InitDevice", &["construct-fail", "1"], &[]))
            .is_err()
    );
    assert!(!runtime.is_initialized());
    assert!(
        EVENTS
            .lock()
            .unwrap()
            .contains(&"drop:before-failure".to_owned())
    );
    runtime
        .invoke(&request("InitDevice", &["recovered", "1"], &[]))
        .unwrap();
    assert_eq!(
        runtime
            .invoke(&request("计数", &["recovered", "1"], &[]))
            .unwrap()
            .as_slice(),
        ["recovered:1"]
    );
}

#[test]
fn cleanup_failure_blocks_new_construction_and_clears_old_object() {
    let runtime = NodeRuntime::<Singleton>::new();
    runtime
        .invoke(&request("InitDevice", &["close-fail", "1"], &[]))
        .unwrap();
    assert!(
        runtime
            .invoke(&request("InitDevice", &["must-not-create", "1"], &[]))
            .is_err()
    );
    assert!(!runtime.is_initialized());
    let events = EVENTS.lock().unwrap();
    assert!(events.contains(&"drop:close-fail".to_owned()));
    assert!(!events.contains(&"create:must-not-create".to_owned()));
}

#[test]
fn enabling_feature_alone_does_not_enable_multi_channel_calls() {
    let runtime = NodeRuntime::<Singleton>::new();
    runtime
        .invoke(&request("InitDevice", &["ordinary", "1"], &[]))
        .unwrap();
    assert!(matches!(
        runtime.invoke(&request("计数", &["ordinary", "1"], &["共享", "", ""])),
        Err(MiresidianError::NotSupported(_))
    ));
    assert_eq!(
        runtime
            .invoke(&request("计数", &["ordinary", "1"], &[]))
            .unwrap()
            .as_slice(),
        ["ordinary:1"]
    );
}

#[test]
fn equivalent_typed_config_finds_and_reuses_the_single_object() {
    let runtime = NodeRuntime::<Singleton>::new();
    runtime
        .invoke(&request("InitDevice", &["canonical-single", "1"], &[]))
        .unwrap();
    assert_eq!(
        runtime
            .invoke(&request("InitDevice", &["canonical-single", "0x1"], &[]))
            .unwrap()
            .as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    assert_eq!(
        runtime
            .invoke(&request("计数", &["canonical-single", "01"], &[]))
            .unwrap()
            .as_slice(),
        ["canonical-single:1"]
    );
    runtime
        .invoke(&request("CloseDevice", &["canonical-single", "0b1"], &[]))
        .unwrap();
}
