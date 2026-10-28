#![cfg(feature = "lease")]

use miresidian::{
    CLOSE_DEVICE_OPERATION, INIT_DEVICE_OPERATION, Invocation, MiresidianResult, NodeRuntime,
};

#[miresidian::node("声明式租约节点", lease(
    activation = required,
    resource = "port_name",
    configuration = ["baud_rate", "data_bits"]
))]
struct DeclarativeLeaseNode {}

impl DeclarativeLeaseNode {
    #[miresidian::operation]
    #[miresidian::output("状态")]
    fn init_device(
        #[miresidian::config("端口名称")] port_name: String,
        #[miresidian::config("波特率")] baud_rate: u32,
        #[miresidian::config("数据位")] data_bits: u8,
    ) -> MiresidianResult<(Self, String)> {
        let _ = (port_name, baud_rate, data_bits);
        Ok((Self {}, "已初始化".to_owned()))
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}

#[test]
fn struct_lease_declaration_derives_resource_and_configuration() {
    let compatible = vec!["COM7".to_owned(), "9600".to_owned(), "8".to_owned()];
    let incompatible = vec!["COM7".to_owned(), "4800".to_owned(), "8".to_owned()];
    let mut runtime = NodeRuntime::<DeclarativeLeaseNode>::new();
    runtime
        .invoke(&Invocation::new(
            INIT_DEVICE_OPERATION,
            compatible.clone(),
            Vec::new(),
        ))
        .expect("声明式租约应先获取再初始化");

    let mut independent_runtime = NodeRuntime::<DeclarativeLeaseNode>::new();
    independent_runtime
        .invoke(&Invocation::new(
            INIT_DEVICE_OPERATION,
            compatible.clone(),
            Vec::new(),
        ))
        .expect("独立运行时应取得兼容的共享租约");
    assert!(
        independent_runtime
            .invoke(&Invocation::new(
                INIT_DEVICE_OPERATION,
                incompatible.clone(),
                Vec::new(),
            ))
            .is_err(),
        "独立运行时也必须看到已存在的不兼容租约"
    );

    runtime
        .invoke(&Invocation::new(
            CLOSE_DEVICE_OPERATION,
            compatible.clone(),
            Vec::new(),
        ))
        .expect("关闭后应释放首个租约");
    independent_runtime
        .invoke(&Invocation::new(
            CLOSE_DEVICE_OPERATION,
            compatible,
            Vec::new(),
        ))
        .expect("独立运行时关闭后应释放共享租约");
}
