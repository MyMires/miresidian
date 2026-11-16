struct Node;
impl Node {
    #[miresidian::operation]
    #[miresidian::output("结果", encoding(formats = []))]
    fn query(&mut self) -> miresidian::MiresidianResult<Vec<u8>> { Ok(vec![]) }
}
fn main() {}
