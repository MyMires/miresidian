use miresidian::operation;
struct BadExecutionNode;
impl BadExecutionNode {
    #[operation("查询", concurrent)]
    fn query(&mut self) -> miresidian::MiresidianResult<()> { Ok(()) }
}
fn main() {}
