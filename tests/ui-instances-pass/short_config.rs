use miresidian::{node, operation, MiresidianResult};
#[node("实例短写", instances(multiple, config))]
struct Example;
impl Example {
    #[operation]
    fn init_device(name: String) -> MiresidianResult<Self> { let _ = name; Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {}
