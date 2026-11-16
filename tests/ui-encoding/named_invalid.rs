struct Node;
impl Node {
    #[miresidian::operation]
    fn empty(&mut self, #[miresidian::input("数据", encoding("", utf8, hex))] data: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn duplicate(&mut self, #[miresidian::input("数据", encoding("格式", utf8, hex, selector = "格式"))] data: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn misplaced(&mut self, #[miresidian::input("数据", encoding(utf8, hex, "格式"))] data: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn typo(&mut self, #[miresidian::input("数据", encoding(uft8, hex))] data: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn mixed(&mut self, #[miresidian::input("数据", encoding("格式", formats = [utf8, hex]))] data: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn legacy(&mut self, #[miresidian::input("数据", format(hex))] data: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn invalid_auto(&mut self, #[miresidian::input("数据/内容", encoding(utf8, hex))] data: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    fn collision(&mut self, #[miresidian::input("数据", encoding(utf8, hex))] data: String, #[miresidian::input("数据格式")] encoding: String) -> miresidian::MiresidianResult<()> { Ok(()) }
    #[miresidian::operation]
    #[miresidian::output("数据", encoding(utf8, hex))]
    fn output_collision(&mut self, #[miresidian::input("数据", encoding(utf8, hex))] data: String) -> miresidian::MiresidianResult<String> { Ok(data) }
}
fn main() {}
