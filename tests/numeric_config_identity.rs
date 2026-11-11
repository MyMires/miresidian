#![cfg(feature = "instance")]
//! 原类型及别名决定主键规范化；空身份仍先于类型解析跳过。
use miresidian::{Invocation, MiresidianResult, NodeRuntime};
type NumericKey = u16;
#[miresidian::node("数值主键",instance(mode = multiple,binding = config,execution = sequential))]
struct NumericNode {
    key: NumericKey,
}
impl NumericNode {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("主键:")] key: NumericKey) -> MiresidianResult<Self> {
        Ok(Self { key })
    }
    #[miresidian::operation("Read")]
    fn read(&mut self) -> MiresidianResult<NumericKey> {
        Ok(self.key)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
#[test]
fn numeric_alias_identity_is_canonical_and_blank_does_not_create_zero() {
    let runtime = NodeRuntime::<NumericNode>::new();
    let request = |op: &str, key: &str| Invocation::new(op, vec![key.into()], vec![]);
    runtime.invoke(&request("InitDevice", " ")).unwrap();
    assert!(!runtime.is_initialized());
    runtime.invoke(&request("InitDevice", "0")).unwrap();
    assert_eq!(
        runtime
            .invoke(&request("InitDevice", "0x0"))
            .unwrap()
            .as_slice(),
        [miresidian::CHANNEL_REUSED_MESSAGE]
    );
    assert_eq!(
        runtime
            .invoke(&request("Read", "主键:0b0"))
            .unwrap()
            .as_slice(),
        ["0"]
    );
    runtime.invoke(&request("CloseDevice", "00")).unwrap();
    assert!(!runtime.is_initialized());
}
