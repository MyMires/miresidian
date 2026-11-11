//! 独占文件模拟物理端口；通过不同 Handle 和不同名 DLL 验证资源共用。
#[path = "support/shared_resource.rs"]
mod transport;

use miresidian::{MiresidianResult, lease::shared::SharedResource};

#[miresidian::node("共享资源节点 A", instance(mode = multiple, binding = handle, execution = sequential))]
struct NodeA {
    resource: SharedResource,
    calls: usize,
}

impl NodeA {
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
        transport::query(&self.resource, &mut self.calls, "A", &text)
    }
}
