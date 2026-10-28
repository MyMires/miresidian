use miresidian::operation;
struct Conflicting;
impl Conflicting {
    #[operation]
    fn query(&mut self, #[miresidian::config("配置")] #[miresidian::input("输入")] value: u32) -> miresidian::MiresidianResult<()> { let _ = value; Ok(()) }
}
fn main() {}
