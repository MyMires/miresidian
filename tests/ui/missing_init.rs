use miresidian::{node, operation};
#[node("缺少初始化")]
struct MissingInit;
impl MissingInit {
    #[operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> { Ok(()) }
}
fn main() {}
