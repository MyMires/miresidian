use miresidian::{node, operation, MiresidianResult};
#[node("实例短写", instance(parallel))]
struct Example;
impl Example {
    #[operation]
    fn init_device() -> MiresidianResult<Self> {  Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {}
