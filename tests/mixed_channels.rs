//! 验证手动标注可逐参数覆盖自动 Config/Input 来源。

use miresidian::{
    DeviceNode, GET_FUNCTIONS_OPERATION, GET_INIT_CONFIG_OPERATION, Invocation, InvocationOutput,
    MiresidianResult, NodeRuntime, Platform, PlatformAdapter, RuntimeInvocation, RuntimeResponse,
};

#[derive(Clone, Copy, Default)]
struct MacroPlatformAdapter;

impl PlatformAdapter for MacroPlatformAdapter {
    fn platform(&self) -> Platform {
        Platform::Custom(0x0001_0001)
    }

    fn identifier(&self) -> &'static str {
        "macro_platform"
    }

    fn decode(&self, invocation: Invocation) -> MiresidianResult<RuntimeInvocation> {
        Ok(RuntimeInvocation::Single(invocation))
    }

    fn encode(&self, response: RuntimeResponse) -> MiresidianResult<InvocationOutput> {
        match response {
            RuntimeResponse::Output(output) => Ok(output),
            RuntimeResponse::Shared(_) => Ok(InvocationOutput::empty()),
        }
    }
}

#[miresidian::node("双通道测试节点", platforms(MacroPlatformAdapter))]
#[derive(Default)]
struct MixedChannelNode;

#[miresidian::node_impl]
impl MixedChannelNode {
    fn init_device(
        &mut self,
        #[miresidian::config("设备地址")] address: String,
        #[miresidian::config("超时毫秒")] timeout_ms: u64,
        #[miresidian::input("初始化令牌")] token: String,
    ) -> MiresidianResult<String> {
        Ok(format!("{address}:{timeout_ms}:{token}"))
    }

    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("混合查询")]
    fn query(
        &mut self,
        #[miresidian::input("命令")] command: String,
        #[miresidian::config("设备地址")] _address: String,
        #[miresidian::config("超时毫秒")] timeout_ms: u64,
    ) -> MiresidianResult<String> {
        Ok(format!("{command}:{timeout_ms}"))
    }
}

#[test]
fn 手动标注决定每个参数的外部来源通道() {
    let registry = <MixedChannelNode as DeviceNode>::platform_registry();
    assert!(registry.adapter(Platform::Custom(0x0001_0001)).is_some());
    let mut runtime = NodeRuntime::<MixedChannelNode>::with_platforms(registry);
    let init_config = runtime
        .invoke(&Invocation::new(
            GET_INIT_CONFIG_OPERATION,
            Vec::new(),
            Vec::new(),
        ))
        .unwrap();
    assert_eq!(
        init_config.as_slice(),
        [
            "设备地址",
            "超时毫秒",
            "平台适配:macro_platform/tb_lv_test_platform",
            "共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播",
        ]
    );
    let functions = runtime
        .invoke(&Invocation::new(
            GET_FUNCTIONS_OPERATION,
            Vec::new(),
            Vec::new(),
        ))
        .unwrap();
    assert!(functions.as_slice().contains(
        &"InitDevice(设备地址,超时毫秒,平台适配:macro_platform/tb_lv_test_platform,共享执行方式:Sequential-顺序/Parallel-并行/Broadcast-广播,初始化令牌)(返回值)".to_owned()
    ));

    let initialized = runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec!["COM9".to_owned(), "2500".to_owned(), "Parallel".to_owned()],
            vec!["令牌".to_owned()],
        ))
        .unwrap();
    assert_eq!(initialized.as_slice(), ["COM9:2500:令牌"]);

    let execution_conflict = runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec![
                "COM10".to_owned(),
                "2500".to_owned(),
                "Sequential".to_owned(),
            ],
            vec!["令牌".to_owned()],
        ))
        .expect_err("首个通道选择的 Parallel 必须被后续通道保持");
    assert!(execution_conflict.to_string().contains("Parallel"));

    let result = runtime
        .invoke(&Invocation::new(
            "混合查询",
            vec!["COM9".to_owned(), "2500".to_owned()],
            vec!["*IDN?".to_owned()],
        ))
        .unwrap();
    assert_eq!(result.as_slice(), ["*IDN?:2500"]);
}
