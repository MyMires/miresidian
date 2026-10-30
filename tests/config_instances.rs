#![cfg(feature = "instances")]
//! Config[0] 是业务主键；其余配置不能使空主键触发构造。

use miresidian::{Invocation, MiresidianError, MiresidianResult, NodeRuntime};

#[miresidian::node("Config 主键", instances(mode = multiple, binding = config, execution = sequential))]
struct ConfigNode {
    key: String,
    speed: u32,
    calls: usize,
}

impl ConfigNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("主键:default")] key: String,
        #[miresidian::config("速度:9600")] speed: u32,
    ) -> MiresidianResult<Self> {
        Ok(Self {
            key,
            speed,
            calls: 0,
        })
    }
    #[miresidian::operation("计数")]
    fn count(&mut self) -> MiresidianResult<String> {
        self.calls += 1;
        Ok(format!("{}:{}:{}", self.key, self.speed, self.calls))
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}

fn request(operation: &str, config: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        vec![],
    )
}

#[test]
fn blank_first_config_skips_even_if_other_business_or_framework_fields_are_nonempty() {
    let runtime = NodeRuntime::<ConfigNode>::new();
    for config in [
        vec![],
        vec!["", "9600"],
        vec![" ", "invalid-speed"],
        vec!["主键:", "9600", "tb_lv_test_platform"],
    ] {
        assert!(
            runtime
                .invoke(&request("InitDevice", &config))
                .unwrap()
                .as_slice()
                .is_empty()
        );
        assert!(!runtime.is_initialized());
    }
    runtime
        .invoke(&request("InitDevice", &["A", "9600"]))
        .unwrap();
    runtime
        .invoke(&request("InitDevice", &["", "4800"]))
        .unwrap();
    assert_eq!(
        runtime
            .invoke(&request("计数", &["A", "9600"]))
            .unwrap()
            .as_slice(),
        ["A:9600:1"]
    );
}

#[test]
fn first_config_is_the_identity_and_conflicting_settings_do_not_create_another_object() {
    let runtime = NodeRuntime::<ConfigNode>::new();
    runtime
        .invoke(&request("InitDevice", &["A", "9600"]))
        .unwrap();
    assert_eq!(
        runtime
            .invoke(&request("InitDevice", &["主键:A", "9600"]))
            .unwrap()
            .as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    assert!(
        matches!(runtime.invoke(&request("InitDevice", &["A", "4800"])), Err(MiresidianError::InvalidArgument { field, .. }) if field == "Config[0]")
    );
    runtime
        .invoke(&request("InitDevice", &["B", "9600"]))
        .unwrap();
    // 非主键字段不用于查找；方法读取对象已保存的配置。
    assert_eq!(
        runtime
            .invoke(&request("计数", &["A", "4800"]))
            .unwrap()
            .as_slice(),
        ["A:9600:1"]
    );
    assert_eq!(
        runtime
            .invoke(&request("计数", &["B", "9600"]))
            .unwrap()
            .as_slice(),
        ["B:9600:1"]
    );
    runtime
        .invoke(&request("CloseDevice", &["A", "4800"]))
        .unwrap();
    assert!(runtime.invoke(&request("计数", &["A", "9600"])).is_err());
    assert_eq!(
        runtime
            .invoke(&request("计数", &["B", "9600"]))
            .unwrap()
            .as_slice(),
        ["B:9600:2"]
    );
}
