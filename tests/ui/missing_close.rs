use miresidian::{node, operation};
#[node("缺少关闭")]
struct MissingClose;
impl MissingClose {
    #[operation]
    fn init_device() -> miresidian::MiresidianResult<Self> { Ok(Self) }
}
fn main() {}
