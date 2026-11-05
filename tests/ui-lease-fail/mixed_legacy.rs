use miresidian::{node, operation, MiresidianResult};
#[node("角色验证", lease(activation = required, resource = "resource", configuration = ["resource"]))]
struct Node;
impl Node {
    #[operation]
    fn init_device(#[miresidian::config("资源", lease(key))] resource: String) -> MiresidianResult<Self> { let _ = resource; Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {}
