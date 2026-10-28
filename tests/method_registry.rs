//! 方法独立注册、构造式初始化和资源失败清理的公开契约回归。
use miresidian::{
    DeviceNode, DeviceOutput, Invocation, InvocationOutput, MiresidianError, MiresidianResult,
    NodeRuntime,
};
use std::sync::Mutex;

static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

// Deliberately has no Default implementation.
struct Resource(String);
impl Drop for Resource {
    fn drop(&mut self) {
        EVENTS.lock().unwrap().push(format!("drop:{}", self.0));
        assert!(!self.0.contains("drop-panic"), "resource drop panic");
    }
}

struct InitOutput(String);
impl DeviceOutput for InitOutput {
    fn into_device_output(self) -> MiresidianResult<InvocationOutput> {
        if self.0.starts_with("encoding-error") {
            return Err(MiresidianError::Failure("encoding failed".into()));
        }
        assert!(!self.0.starts_with("encoding-panic"), "encoding panic");
        Ok(InvocationOutput::new(vec!["ready".to_owned()]))
    }
}

#[miresidian::node("独立注册测试")]
struct RegisteredNode {
    resource: Resource,
}

impl RegisteredNode {
    #[miresidian::operation]
    #[miresidian::output("状态")]
    fn init_device(
        #[miresidian::config("地址:ok")] address: String,
    ) -> MiresidianResult<(Self, InitOutput)> {
        let resource = Resource(address.clone());
        if address == "construct-error" {
            return Err(MiresidianError::Failure("construction failed".into()));
        }
        assert_ne!(address, "construct-panic", "constructor panic");
        Ok((Self { resource }, InitOutput(address)))
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        EVENTS
            .lock()
            .unwrap()
            .push(format!("close:{}", self.resource.0));
        assert!(!self.resource.0.contains("close-panic"), "close panic");
        Ok(())
    }

    // Reversed attribute order retains the existing output annotation semantics.
    #[miresidian::output("回传")]
    #[miresidian::operation("ZFirst")]
    fn echo(
        &mut self, #[miresidian::input("文本:默认")] text: String
    ) -> MiresidianResult<String> {
        self.helper();
        Ok(text)
    }

    fn helper(&self) {}
}

impl RegisteredNode {
    #[miresidian::operation("ASecond")]
    fn second(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("Disabled")]
    #[cfg(any())]
    fn disabled(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("DisabledAttr")]
    #[cfg_attr(all(), cfg(any()))]
    fn disabled_attr(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}

mod disabled {
    #[miresidian::node("禁用节点")]
    #[cfg(any())]
    struct Disabled;
}

fn init(address: &str) -> Invocation {
    Invocation::new("InitDevice", vec![address.to_owned()], vec![])
}

#[test]
fn multiple_impls_publish_source_order_and_preserve_parameter_annotations() {
    let descriptor = RegisteredNode::descriptor();
    assert_eq!(
        descriptor
            .operations
            .iter()
            .map(|operation| operation.name)
            .collect::<Vec<_>>(),
        ["ZFirst", "ASecond"]
    );
    assert_eq!(descriptor.init_outputs[0].name, "状态");
    assert_eq!(
        descriptor.operations[0].signature(),
        "ZFirst(文本:默认)(回传)"
    );
    let mut runtime = NodeRuntime::<RegisteredNode>::new();
    assert_eq!(
        runtime.invoke(&init("registered")).unwrap().as_slice(),
        ["ready"]
    );
    assert_eq!(
        runtime
            .invoke(&Invocation::new(
                "ZFirst",
                vec!["registered".into()],
                vec![]
            ))
            .unwrap()
            .as_slice(),
        ["默认"]
    );
    assert_eq!(
        runtime.invoke(&init("registered")).unwrap().as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    runtime.shutdown().unwrap();
}

#[test]
fn constructors_clean_their_resources_without_runtime_close() {
    for address in ["construct-error", "construct-panic"] {
        let mut runtime = NodeRuntime::<RegisteredNode>::new();
        assert!(runtime.invoke(&init(address)).is_err());
        assert!(!runtime.is_initialized());
        let events = EVENTS.lock().unwrap();
        assert!(events.contains(&format!("drop:{address}")));
        assert!(!events.contains(&format!("close:{address}")));
    }
}

#[test]
fn output_failure_closes_and_drops_the_returned_node_even_if_cleanup_panics() {
    for address in [
        "encoding-error",
        "encoding-panic",
        "encoding-panic-close-panic-drop-panic",
    ] {
        let mut runtime = NodeRuntime::<RegisteredNode>::new();
        let error = runtime.invoke(&init(address)).unwrap_err();
        assert_eq!(
            error.code(),
            if address == "encoding-error" {
                "OPERATION_FAILED"
            } else {
                "NODE_PANICKED"
            }
        );
        assert!(!runtime.is_initialized());
        assert!(
            runtime
                .invoke(&Invocation::new("GetFunctions", vec![], vec![]))
                .is_ok()
        );
        let events = EVENTS.lock().unwrap();
        let close = events
            .iter()
            .position(|event| event == &format!("close:{address}"))
            .unwrap();
        let drop = events
            .iter()
            .position(|event| event == &format!("drop:{address}"))
            .unwrap();
        assert!(close < drop);
    }
}
