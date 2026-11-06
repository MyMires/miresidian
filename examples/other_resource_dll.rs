//! 不同业务节点独立编译，使用相同通信层事务协议共用物理资源。
//! 每个 NodeB 保留自己的 calls；SharedResource 仅共享底层连接及完整事务互斥。
//! 与 shared_resource_dll 配套验证；独占文件模拟设备，不代表真实串口通信已验证。
#[path = "support/shared_resource.rs"]
mod transport;

use miresidian::{MiresidianResult, lease::shared::SharedResource};

#[miresidian::node("共享资源节点 B")]
struct NodeB {
    resource: SharedResource,
    calls: usize,
}

impl NodeB {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("资源")] resource: String) -> MiresidianResult<Self> {
        transport::connect(&resource).map(|resource| Self { resource, calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        self.resource.close().map_err(transport::error)
    }
    #[miresidian::operation("查询")]
    fn query(&mut self, #[miresidian::input("消息")] text: String) -> MiresidianResult<String> {
        transport::query(&self.resource, &mut self.calls, "B", &text)
    }
}
