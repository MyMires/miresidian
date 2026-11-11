use miresidian::{node, operation, MiresidianResult};
#[node("租约短写", lease(selectable(required)))]
struct Example;
impl Example {
    #[operation]
    fn init_device(#[miresidian::config("资源:demo", lease(key))] resource: String) -> MiresidianResult<Self> { let _ = resource; Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {}
