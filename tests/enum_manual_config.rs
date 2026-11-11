//! 手写显示值 Config 与派生 codec 的规范身份保持一致。
use miresidian::{Invocation, MiresidianResult, NodeRuntime};

#[derive(miresidian::NodeEnum)]
enum State {
    Idle,
    Running,
}

#[miresidian::node("枚举配置身份")]
struct Workflow;
impl Workflow {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("状态:Idle-空闲/Running-运行中")] _state: State,
    ) -> MiresidianResult<Self> {
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation("回传")]
    fn echo(&mut self, value: State) -> MiresidianResult<State> {
        Ok(value)
    }
}

#[test]
fn explicit_display_and_actual_config_reuse_one_object() {
    let runtime = NodeRuntime::<Workflow>::new();
    let call = |operation: &str, config: &str, input: Vec<String>| {
        runtime.invoke(&Invocation::new(operation, vec![config.to_owned()], input))
    };
    call("InitDevice", "空闲", vec![]).unwrap();
    assert_eq!(
        call("InitDevice", "状态:Idle", vec![]).unwrap().as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    assert_eq!(
        call("回传", "Idle", vec!["Running".into()])
            .unwrap()
            .as_slice(),
        ["Running"]
    );
}
