use miresidian::operation;
struct InvalidOutput;
impl InvalidOutput {
    #[operation]
    #[miresidian::output("结果")]
    fn query(&mut self) -> miresidian::MiresidianResult<(String, String)> {
        Ok((String::new(), String::new()))
    }
}
fn main() {}
