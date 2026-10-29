#![cfg(all(feature = "instances", feature = "lease"))]
//! Handle、租约、平台、执行方式共用一套 Config 布局。

use miresidian::{Invocation, MiresidianResult, NodeRuntime, TbLvTestPlatformAdapter};

#[miresidian::node("Handle 租约布局",
    instances(mode = multiple, binding = handle, execution = selectable, default_execution = sequential),
    lease(activation = selectable, resource = "resource", configuration = ["setting"])
)]
struct LeasedHandleNode {
    setting: u32,
    calls: usize,
}

impl LeasedHandleNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("资源")] resource: String,
        #[miresidian::config("配置")] setting: u32,
    ) -> MiresidianResult<Self> {
        let _ = resource;
        Ok(Self { setting, calls: 0 })
    }
    #[miresidian::operation("计数")]
    fn count(
        &mut self,
        #[miresidian::config("资源")] _resource: String,
        #[miresidian::config("配置")] setting: u32,
    ) -> MiresidianResult<String> {
        assert_eq!(setting, self.setting);
        self.calls += 1;
        Ok(format!("{setting}:{}", self.calls))
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
fn selectable_lease_and_execution_are_published_after_handle_and_parsed_consistently() {
    for explicit in [false, true] {
        let mut runtime = NodeRuntime::<LeasedHandleNode>::new();
        let mut call = |operation, config: &[&str]| {
            let request = request(operation, config);
            if explicit {
                runtime.invoke_with(&TbLvTestPlatformAdapter, request)
            } else {
                runtime.invoke(&request)
            }
        };
        let definitions = call("GetInitConfig", &[]).unwrap().into_values();
        assert_eq!(
            &definitions[..4],
            [
                "资源",
                "配置",
                "Handle",
                "资源访问方式:Lease-租约/Direct-直连"
            ]
        );
        assert_eq!(
            definitions.last().unwrap(),
            "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播"
        );
        assert_eq!(definitions.len(), if explicit { 5 } else { 6 });
        let resource = format!("handle-layout:{}:{explicit}", std::process::id());
        for handle in ["one", "two"] {
            let mut config = vec![resource.as_str(), "1", handle, "Lease"];
            if !explicit {
                config.push("tb_lv_test_platform");
            }
            config.push("Parallel");
            call("InitDevice", &config).unwrap();
            assert_eq!(
                call("计数", &[&resource, "1", handle]).unwrap().as_slice(),
                ["1:1"]
            );
        }
        call("CloseAllDevices", &[]).unwrap();
    }
}
