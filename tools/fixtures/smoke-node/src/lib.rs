//! Windows 工具链端到端冒烟夹具。

use miresidian::{MiresidianResult, node};

/// 不访问硬件，只验证框架生成的 ABI 与工具调用链。
#[node("工具冒烟节点")]
struct ToolSmokeNode;

impl ToolSmokeNode {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        Ok(Self)
    }

    #[miresidian::operation("回显")]
    fn echo(
        &mut self, #[miresidian::input("发送文本")] text: String
    ) -> MiresidianResult<String> {
        Ok(text)
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
}
