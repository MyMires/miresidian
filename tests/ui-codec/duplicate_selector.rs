struct Node;
impl Node {
    #[miresidian::operation]
    fn query(&mut self,
        #[miresidian::input("数据", codec(formats = [hex, utf8], selector = "格式"))] data: Vec<u8>,
        #[miresidian::input("格式")] format: String,
    ) -> miresidian::MiresidianResult<()> { Ok(()) }
}
fn main() {}
