use miresidian::operation;
struct InvalidInit;
impl InvalidInit {
    #[operation]
    fn init_device(&mut self) -> miresidian::MiresidianResult<Self> { Ok(Self) }
}
fn main() {}
