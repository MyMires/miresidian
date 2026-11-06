#![cfg(feature = "lease")]
use miresidian::{Invocation, MiresidianError, NodeRuntime};
static CLOSED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
#[miresidian::node("最后关闭参数", lease(activation = required))]
struct CloseNode;
impl CloseNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("资源", lease(key))] resource: String,
    ) -> miresidian::MiresidianResult<Self> {
        let _ = resource;
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(
        &mut self,
        #[miresidian::input("关闭参数")] value: String,
    ) -> miresidian::MiresidianResult<()> {
        CLOSED.lock().unwrap().push(value.clone());
        if value == "fail" {
            return Err(MiresidianError::InvalidArgument {
                field: "关闭参数".into(),
                message: "关闭失败".into(),
            });
        }
        if value == "panic" {
            panic!("close panic");
        }
        Ok(())
    }
}
#[test]
fn only_final_close_receives_its_input_and_preserves_errors_after_cleanup() {
    for value in ["last", "fail", "panic"] {
        let name = format!("close-args:{}:{value}", std::process::id());
        let a = NodeRuntime::<CloseNode>::new();
        let b = NodeRuntime::<CloseNode>::new();
        let call = |runtime: &NodeRuntime<CloseNode>, op: &str, input: &[&str]| {
            runtime.invoke(&Invocation::new(
                op,
                vec![name.clone()],
                input.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
            ))
        };
        call(&a, "InitDevice", &[]).unwrap();
        call(&b, "InitDevice", &[]).unwrap();
        let before = CLOSED.lock().unwrap().len();
        call(&a, "CloseDevice", &["not-last"]).unwrap();
        assert_eq!(CLOSED.lock().unwrap().len(), before);
        let closed = call(&b, "CloseDevice", &[value]);
        match value {
            "fail" => assert!(matches!(
                closed,
                Err(MiresidianError::InvalidArgument { .. })
            )),
            "panic" => assert!(matches!(closed, Err(MiresidianError::NodePanicked { .. }))),
            _ => {
                closed.unwrap();
            }
        }
        assert_eq!(CLOSED.lock().unwrap().last().unwrap(), value);
        assert!(!a.is_initialized() && !b.is_initialized());
        // Even a failing close must release the old owner and permit initialization.
        call(&a, "InitDevice", &[]).unwrap();
        call(&a, "CloseDevice", &["cleanup"]).unwrap();
    }
}
