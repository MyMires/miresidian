struct Node;
impl Node {
    #[miresidian::operation]
    fn missing(&mut self, #[miresidian::input("数据", format(utf8, hex))] data: Vec<u8>) -> miresidian::MiresidianResult<Vec<u8>> { Ok(data) }
    #[miresidian::operation]
    fn fixed_selector(&mut self, #[miresidian::input("数据", format(hex, selector = "格式"))] data: Vec<u8>) -> miresidian::MiresidianResult<Vec<u8>> { Ok(data) }
    #[miresidian::operation]
    fn mixed(&mut self, #[miresidian::input("数据", format(hex, formats = [hex]))] data: Vec<u8>) -> miresidian::MiresidianResult<Vec<u8>> { Ok(data) }
}
fn main() {}
