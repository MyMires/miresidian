use miresidian::{node, operation, MiresidianResult};
#[node("角色验证", instances(mode = multiple, binding = config, execution = sequential))]
struct Node;
impl Node {
    #[operation]
    fn init_device(#[miresidian::config("资源", instance(ignore))] resource: String) -> MiresidianResult<Self> { let _ = resource; Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {}
