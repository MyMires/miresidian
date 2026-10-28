use miresidian::operation;
struct Generic;
impl Generic {
    #[operation]
    fn query<T>(&mut self, value: T) -> miresidian::MiresidianResult<()> { let _ = value; Ok(()) }
}
fn main() {}
