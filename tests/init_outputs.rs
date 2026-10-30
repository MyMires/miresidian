//! 构造结果中的节点不进入平台输出，业务元组仍沿用原有 output 规则。
use miresidian::{DeviceNode, Invocation, MiresidianResult, NodeRuntime};

#[miresidian::node("初始化多输出")]
struct OutputNode;
impl OutputNode {
    #[miresidian::operation]
    #[miresidian::output("连接状态", "编号")]
    fn init_device(
        #[miresidian::config("编号:7")] number: u32,
    ) -> MiresidianResult<(Self, (String, u32))> {
        Ok((Self, ("ready".to_owned(), number)))
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
#[test]
fn initialization_only_encodes_and_describes_the_business_tuple() {
    let descriptor = OutputNode::descriptor();
    assert_eq!(
        descriptor
            .init_outputs
            .iter()
            .map(|p| p.name)
            .collect::<Vec<_>>(),
        ["连接状态", "编号"]
    );
    let runtime = NodeRuntime::<OutputNode>::new();
    let output = runtime
        .invoke(&Invocation::new("InitDevice", vec![], vec![]))
        .unwrap();
    assert_eq!(output.as_slice(), ["ready", "7"]);
    runtime.shutdown().unwrap();
}
