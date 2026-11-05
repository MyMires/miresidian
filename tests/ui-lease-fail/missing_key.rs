use miresidian::{node, operation, MiresidianResult};
#[node("角色验证", lease(activation = required))]
struct Node;
impl Node {
    #[operation]
    fn init_device(#[miresidian::config("资源")] resource: String) -> MiresidianResult<Self> { let _ = resource; Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {}
