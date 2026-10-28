use miresidian::operation;
struct InvalidInit;
impl InvalidInit {
    #[operation]
    fn init_device() -> miresidian::MiresidianResult<String> { Ok(String::new()) }
}
fn main() {}
