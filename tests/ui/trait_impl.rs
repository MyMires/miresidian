use miresidian::{node, operation};
trait ExternalContract { fn query(&mut self) -> miresidian::MiresidianResult<()>; }
#[node("非法 trait 实现")]
struct TraitNode;
impl TraitNode {
    #[operation]
    fn init_device() -> miresidian::MiresidianResult<Self> { Ok(Self) }
    #[operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> { Ok(()) }
}
impl ExternalContract for TraitNode {
    #[operation]
    fn query(&mut self) -> miresidian::MiresidianResult<()> { Ok(()) }
}
fn main() {}
