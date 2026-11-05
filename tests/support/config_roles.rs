#![cfg(feature = "instances")]
//! Shared end-to-end assertions; each test binary exports exactly one node.
use miresidian::{DeviceNode, Invocation, MiresidianError, NodeRuntime};

pub fn request(operation: &str, config: &[&str], input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        input.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )
}

macro_rules! fixture {
    ($module:ident, [$($timeout:tt)*], [$($ip:tt)*], [$($port:tt)*]) => {
        mod $module {
            use miresidian::MiresidianResult;
            #[miresidian::node("组合身份", instances(mode = multiple, binding = config, execution = sequential))]
            pub struct Node { timeout: u64, ip: String, port: u16, count: usize }
            impl Node {
                #[miresidian::operation]
                fn init_device(
                    #[miresidian::config("超时:", $($timeout)*)] timeout: u64,
                    #[miresidian::config("IP", $($ip)*)] ip: String,
                    #[miresidian::config("端口:", $($port)*)] port: u16,
                ) -> MiresidianResult<Self> { Ok(Self { timeout, ip, port, count: 0 }) }
                #[miresidian::operation]
                async fn count(&mut self) -> MiresidianResult<String> {
                    self.count += 1;
                    Ok(format!("{}:{}:{}:{}", self.ip, self.port, self.timeout, self.count))
                }
                #[miresidian::operation]
                fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
            }
        }
    };
}
pub(crate) use fixture;

pub fn default_keys<N: DeviceNode>() {
    let runtime = NodeRuntime::<N>::new();
    for config in [
        ["100", "host", "8000"],
        ["200", "host", "8000"],
        ["100", "host", "9000"],
    ] {
        runtime
            .invoke(&request("InitDevice", &config, &[]))
            .unwrap();
        assert!(
            runtime
                .invoke(&request("Count", &config, &[]))
                .unwrap()
                .as_slice()[0]
                .ends_with(":1")
        );
    }
    assert!(
        runtime
            .invoke(&request("Count", &["300", "host", "8000"], &[]))
            .is_err()
    );
    assert_eq!(
        runtime
            .invoke(&request("InitDevice", &["0100", "IP:host", "0x1f40"], &[]))
            .unwrap()
            .as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    // Every selected raw key is checked before type parsing, including a nonleading key.
    assert!(
        runtime
            .invoke(&request("InitDevice", &["invalid", "host", " "], &[]))
            .unwrap()
            .as_slice()
            .is_empty()
    );
    runtime
        .invoke(&request("CloseDevice", &["200", "host", "8000"], &[]))
        .unwrap();
    assert!(
        runtime
            .invoke(&request("Count", &["100", "host", "8000"], &[]))
            .is_ok()
    );
}

pub async fn selected_keys<N: DeviceNode>() {
    let runtime = NodeRuntime::<N>::new();
    // The first Config is not an identity field; blank numeric values normalize to zero.
    for port in ["8000", "9000"] {
        runtime
            .invoke(&request("InitDevice", &["", "host", port], &[]))
            .unwrap();
    }
    assert!(matches!(
        runtime.invoke(&request("InitDevice", &["1", "host", "8000"], &[])),
        Err(MiresidianError::InvalidArgument { .. })
    ));
    assert_eq!(
        runtime
            .invoke_async(request("Count", &["unused", "IP:host", "0x1f40"], &[]))
            .await
            .unwrap()
            .as_slice(),
        ["host:8000:0:1"]
    );
    // Structured multi-channel calls select the same object and retain output slots.
    let response = runtime
        .invoke_command_async(miresidian::RuntimeInvocation::MultiChannel(
            miresidian::MultiChannelInvocation::new(
                "Count",
                vec!["invalid".into(), "host".into(), "9000".into()],
                vec![Some(vec![]), Some(vec![])],
            ),
        ))
        .await
        .unwrap();
    let miresidian::RuntimeResponse::MultiChannel(output) = response else {
        panic!("expected channel response")
    };
    assert_eq!(
        output.slots[0].as_ref().unwrap().as_slice(),
        ["host:9000:0:1"]
    );
    assert_eq!(
        output.slots[1].as_ref().unwrap().as_slice(),
        ["host:9000:0:2"]
    );
    assert!(
        runtime
            .invoke(&request("InitDevice", &["bad", "", "8000"], &[]))
            .unwrap()
            .as_slice()
            .is_empty()
    );
    assert!(
        runtime
            .invoke(&request("Count", &["0", "host"], &[]))
            .is_err()
    );
    runtime
        .invoke(&request("CloseDevice", &["unused", "host", "8000"], &[]))
        .unwrap();
    assert!(
        runtime
            .invoke(&request("Count", &["0", "host", "8000"], &[]))
            .is_err()
    );
    assert!(
        runtime
            .invoke(&request("Count", &["0", "host", "9000"], &[]))
            .is_ok()
    );
}
