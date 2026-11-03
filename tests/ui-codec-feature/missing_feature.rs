#[miresidian::node("缺少格式能力")]
struct Node;
impl Node {
    #[miresidian::operation]
    fn init_device() -> miresidian::MiresidianResult<Self> { Ok(Self) }
    #[miresidian::operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn echo(&mut self,
        #[miresidian::input("数据", codec(formats = [hex]))] data: Vec<u8>,
    ) -> miresidian::MiresidianResult<()> { Ok(()) }
}
fn main() {}
