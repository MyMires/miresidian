//! 注册表汇总后的同名平台操作必须在执行任何节点业务前被拒绝。
use miresidian::{Invocation, MiresidianError, MiresidianResult, NodeRuntime};

#[miresidian::node("重复契约测试")]
struct DuplicateNode;
impl DuplicateNode {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        panic!("invalid registry must not construct node")
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation("同名")]
    fn first(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
impl DuplicateNode {
    #[miresidian::operation("同名")]
    fn second(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
#[test]
fn duplicate_names_fail_metadata_queries_and_initialization() {
    let mut runtime = NodeRuntime::<DuplicateNode>::new();
    for operation in ["GetFunctions", "InitDevice"] {
        let error = runtime
            .invoke(&Invocation::new(operation, vec![], vec![]))
            .unwrap_err();
        assert!(matches!(error, MiresidianError::InvalidArgument { .. }));
        assert!(error.to_string().contains("同名"));
        assert!(!runtime.is_initialized());
    }
}
