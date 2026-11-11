//! 保持原 Option 签名，验证平台发现原因及错误边界。
use miresidian::{Invocation, MiresidianError, MiresidianResult, NodeRuntime};

#[miresidian::node("发现原因回归")]
struct Probe;
impl Probe {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        panic!("发现不应初始化节点")
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation]
    fn get_port_no(mode: u32) -> MiresidianResult<Option<String>> {
        match mode {
            0 => Ok(Some("COM7".into())),
            1 => Ok(None),
            2 => Err(MiresidianError::NotSupported("此设备不支持自动发现".into())),
            3 => Err(MiresidianError::NotSupported(" \t".into())),
            _ => Err(MiresidianError::Failure("扫描失败".into())),
        }
    }
}
#[test]
fn probe_output_preserves_addresses_and_explains_misses() {
    let runtime = NodeRuntime::<Probe>::new();
    let call = |mode: u32| {
        runtime.invoke(&Invocation::new(
            "GetPortNO",
            vec![],
            vec![mode.to_string()],
        ))
    };
    for (mode, expected) in [
        (0, ["1", "COM7"]),
        (1, ["0", "未找到匹配设备"]),
        (2, ["0", "此设备不支持自动发现"]),
        (3, ["0", "当前未实现端口发现能力"]),
    ] {
        assert_eq!(call(mode).unwrap().as_slice(), expected);
    }
    assert!(matches!(call(4), Err(MiresidianError::Failure(_))));
    assert!(!runtime.is_initialized());
}
