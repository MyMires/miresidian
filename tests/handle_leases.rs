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
        let runtime = NodeRuntime::<LeasedHandleNode>::new();
        let call = |operation, config: &[&str]| {
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
        for (index, handle) in ["one", "two"].into_iter().enumerate() {
            let mut config = vec![resource.as_str(), "1", handle, "Lease"];
            if !explicit {
                config.push("tb_lv_test_platform");
            }
            config.push("Parallel");
            call("InitDevice", &config).unwrap();
            assert_eq!(
                call("计数", &[&resource, "1", handle]).unwrap().as_slice(),
                [format!("1:{}", index + 1)]
            );
        }
        call("CloseAllDevices", &[]).unwrap();
    }
}

#[test]
fn reused_handle_preserves_state_and_rejects_lease_access_changes() {
    for initial in ["Lease", "Direct"] {
        let runtime = NodeRuntime::<LeasedHandleNode>::new();
        let resource = format!("handle-access:{}:{initial}", std::process::id());
        let config = [&*resource, "1", "one", initial];
        runtime.invoke(&request("InitDevice", &config)).unwrap();
        assert_eq!(
            runtime
                .invoke(&request("计数", &config))
                .unwrap()
                .as_slice(),
            ["1:1"]
        );
        assert_eq!(
            runtime
                .invoke(&request("InitDevice", &config))
                .unwrap()
                .as_slice(),
            [miresidian::CHANNEL_REUSED_MESSAGE]
        );
        let changed = [
            &*resource,
            "1",
            "one",
            if initial == "Lease" {
                "Direct"
            } else {
                "Lease"
            },
        ];
        assert!(
            matches!(runtime.invoke(&request("InitDevice", &changed)), Err(miresidian::MiresidianError::InvalidArgument { field, .. }) if field == "资源访问方式")
        );
        assert_eq!(
            runtime
                .invoke(&request("计数", &config))
                .unwrap()
                .as_slice(),
            ["1:2"]
        );
        runtime.invoke(&request("CloseDevice", &config)).unwrap();
        runtime.invoke(&request("InitDevice", &changed)).unwrap();
        runtime.shutdown().unwrap();
    }
}

#[test]
fn blank_and_explicit_numeric_zero_share_one_configuration_and_lease_in_both_ffi_entries() {
    use std::{
        ffi::{CStr, CString},
        sync::OnceLock,
    };
    for explicit in [false, true] {
        let runtime = OnceLock::<NodeRuntime<LeasedHandleNode>>::new();
        let resource = format!("typed-lease-zero:{}:{explicit}", std::process::id());
        let call = |op: &str, setting: &str, handle: &str| {
            let op = CString::new(op).unwrap();
            let config = CString::new(
                serde_json::to_string(&[resource.as_str(), setting, handle, "Lease"]).unwrap(),
            )
            .unwrap();
            let input = CString::new("[]").unwrap();
            // SAFETY: 输入有效且在调用期间存活；响应在下一次调用前立即解析。
            unsafe {
                let pointer = if explicit {
                    miresidian::ffi_runtime::invoke_with(
                        &runtime,
                        1,
                        op.as_ptr(),
                        config.as_ptr(),
                        input.as_ptr(),
                    )
                } else {
                    miresidian::ffi_runtime::invoke(
                        &runtime,
                        op.as_ptr(),
                        config.as_ptr(),
                        input.as_ptr(),
                    )
                };
                serde_json::from_str::<serde_json::Value>(CStr::from_ptr(pointer).to_str().unwrap())
                    .unwrap()
            }
        };
        assert_eq!(call("InitDevice", "", "H1")["code"], "OK");
        assert_eq!(
            call("计数", "0", "H1")["output"],
            serde_json::json!(["0:1"])
        );
        let reused = call("InitDevice", "0x0", "H1");
        assert_eq!(reused["code"], "OK");
        assert_eq!(
            reused["output"],
            serde_json::json!([miresidian::CHANNEL_REUSED_MESSAGE])
        );
        assert_eq!(call("InitDevice", "0", "H2")["code"], "OK");
        assert_ne!(call("InitDevice", "1", "H3")["code"], "OK");
        assert_eq!(call("CloseDevice", "unused", "H1")["code"], "OK");
        assert_eq!(
            call("计数", "0", "H2")["output"],
            serde_json::json!(["0:2"])
        );
        assert_eq!(call("CloseDevice", "0", "H2")["code"], "OK");
    }
}
