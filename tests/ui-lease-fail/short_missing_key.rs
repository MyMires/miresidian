use miresidian::{node, operation, MiresidianResult};
#[node("租约短写", lease)]
struct Example;
impl Example {
    #[operation]
    fn init_device(#[miresidian::config("资源:demo")] resource: String) -> MiresidianResult<Self> { let _ = resource; Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {}
