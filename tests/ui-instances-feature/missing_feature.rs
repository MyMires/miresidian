use miresidian::{node, operation, MiresidianResult};
#[node("实例策略", instances(mode = single, execution = sequential))]
struct Example;
impl Example {
    #[operation]
    fn init_device() -> MiresidianResult<Self> {  Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> MiresidianResult<()> { Ok(()) }
}
fn main() {  }
