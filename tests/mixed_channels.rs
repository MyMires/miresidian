//! 验证手动标注可逐参数覆盖自动 Config/Input 来源。

use miresidian::{Invocation, MiresidianResult, NodeRuntime};

#[miresidian::node("双通道测试节点")]
#[derive(Default)]
struct MixedChannelNode;

#[miresidian::node_impl]
impl MixedChannelNode {
    fn init_device(
        &mut self,
        #[miresidian::config("设备地址")] address: String,
        #[miresidian::input("初始化令牌")] token: String,
    ) -> MiresidianResult<String> {
        Ok(format!("{address}:{token}"))
    }

    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("混合查询")]
    fn query(
        &mut self,
        #[miresidian::input("命令")] command: String,
        #[miresidian::config("超时毫秒")] timeout_ms: u64,
    ) -> MiresidianResult<String> {
        Ok(format!("{command}:{timeout_ms}"))
    }
}

#[test]
fn 手动标注决定每个参数的外部来源通道() {
    let mut runtime = NodeRuntime::<MixedChannelNode>::new();
    let initialized = runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec!["COM9".to_owned()],
            vec!["令牌".to_owned()],
        ))
        .unwrap();
    assert_eq!(initialized.as_slice(), ["COM9:令牌"]);

    let result = runtime
        .invoke(&Invocation::new(
            "混合查询",
            vec!["2500".to_owned()],
            vec!["*IDN?".to_owned()],
        ))
        .unwrap();
    assert_eq!(result.as_slice(), ["*IDN?:2500"]);
}
