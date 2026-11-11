#[miresidian::node("格式短写")]
struct Node;
impl Node {
    #[miresidian::operation]
    fn init_device() -> miresidian::MiresidianResult<Self> { Ok(Self) }
    #[miresidian::operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    #[miresidian::output("结果", format(hex))]
    fn echo(&mut self, #[miresidian::input("数据", format(hex))] data: Vec<u8>) -> miresidian::MiresidianResult<Vec<u8>> { Ok(data) }
}
fn main() {}
