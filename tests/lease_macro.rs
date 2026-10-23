#![cfg(feature = "lease")]

use std::sync::atomic::{AtomicUsize, Ordering};

use miresidian::{
    CLOSE_DEVICE_OPERATION, INIT_DEVICE_OPERATION, Invocation, MiresidianResult, NodeRuntime,
};

static DECLARATIVE_ACQUIRE_COUNT: AtomicUsize = AtomicUsize::new(0);
static DECLARATIVE_RELEASE_COUNT: AtomicUsize = AtomicUsize::new(0);

#[miresidian::node("声明式租约节点", lease(
    activation = required,
    resource = "port_name",
    configuration = ["baud_rate", "data_bits"]
))]
#[derive(Default)]
struct DeclarativeLeaseNode {
    held: bool,
}

#[miresidian::node_impl]
impl DeclarativeLeaseNode {
    fn lease_acquire(&mut self, request: miresidian::lease::LeaseRequest) -> MiresidianResult<()> {
        assert_eq!(request.resource().as_str(), "COM7");
        assert_eq!(request.value("baud_rate"), Some("9600"));
        assert_eq!(request.value("data_bits"), Some("8"));
        self.held = true;
        DECLARATIVE_ACQUIRE_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn lease_release(&mut self) -> MiresidianResult<()> {
        assert!(self.held);
        self.held = false;
        DECLARATIVE_RELEASE_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    #[miresidian::output("状态")]
    fn init_device(
        &mut self,
        #[miresidian::config("端口名称")] port_name: String,
        #[miresidian::config("波特率")] baud_rate: u32,
        #[miresidian::config("数据位")] data_bits: u8,
    ) -> MiresidianResult<String> {
        let _ = (port_name, baud_rate, data_bits);
        assert!(self.held);
        Ok("已初始化".to_owned())
    }

    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}

#[test]
fn struct_lease_declaration_derives_resource_and_configuration() {
    DECLARATIVE_ACQUIRE_COUNT.store(0, Ordering::SeqCst);
    DECLARATIVE_RELEASE_COUNT.store(0, Ordering::SeqCst);
    let mut runtime = NodeRuntime::<DeclarativeLeaseNode>::new();
    runtime
        .invoke(&Invocation::new(
            INIT_DEVICE_OPERATION,
            vec!["COM7".to_owned(), "9600".to_owned(), "8".to_owned()],
            Vec::new(),
        ))
        .expect("声明式租约应先获取再初始化");
    runtime
        .invoke(&Invocation::new(
            CLOSE_DEVICE_OPERATION,
            vec!["COM7".to_owned(), "9600".to_owned(), "8".to_owned()],
            Vec::new(),
        ))
        .expect("关闭后应释放声明式租约");
    assert_eq!(DECLARATIVE_ACQUIRE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(DECLARATIVE_RELEASE_COUNT.load(Ordering::SeqCst), 1);
}
