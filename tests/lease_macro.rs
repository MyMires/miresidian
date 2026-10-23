#![cfg(feature = "lease")]

use miresidian::{
    CLOSE_DEVICE_OPERATION, INIT_DEVICE_OPERATION, Invocation, MiresidianResult, NodeRuntime,
};

#[miresidian::node("声明式租约节点", lease(
    activation = required,
    resource = "port_name",
    configuration = ["baud_rate", "data_bits"]
))]
#[derive(Default)]
struct DeclarativeLeaseNode {}

#[miresidian::node_impl]
impl DeclarativeLeaseNode {
    #[miresidian::output("状态")]
    fn init_device(
        &mut self,
        #[miresidian::config("端口名称")] port_name: String,
        #[miresidian::config("波特率")] baud_rate: u32,
        #[miresidian::config("数据位")] data_bits: u8,
    ) -> MiresidianResult<String> {
        let _ = (port_name, baud_rate, data_bits);
        Ok("已初始化".to_owned())
    }

    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}

#[test]
fn struct_lease_declaration_derives_resource_and_configuration() {
    let mut runtime = NodeRuntime::<DeclarativeLeaseNode>::new();
    runtime
        .invoke(&Invocation::new(
            INIT_DEVICE_OPERATION,
            vec!["COM7".to_owned(), "9600".to_owned(), "8".to_owned()],
            Vec::new(),
        ))
        .expect("声明式租约应先获取再初始化");
    assert!(
        runtime
            .invoke(&Invocation::new(
                INIT_DEVICE_OPERATION,
                vec!["COM7".to_owned(), "4800".to_owned(), "8".to_owned()],
                Vec::new(),
            ))
            .is_err(),
        "框架必须拒绝同资源的不兼容租约配置"
    );

    runtime
        .invoke(&Invocation::new(
            CLOSE_DEVICE_OPERATION,
            vec!["COM7".to_owned(), "9600".to_owned(), "8".to_owned()],
            Vec::new(),
        ))
        .expect("关闭后应释放声明式租约");
}
