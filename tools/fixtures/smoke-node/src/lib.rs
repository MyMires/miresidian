//! Windows 工具链端到端冒烟夹具。

use miresidian::{MiresidianResult, node, node_impl};

/// 不访问硬件，只验证框架生成的 ABI 与工具调用链。
#[node("工具冒烟节点")]
#[derive(Default)]
struct ToolSmokeNode;

#[node_impl]
impl ToolSmokeNode {
    fn init_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
