use miresidian_macros::node_impl;

struct MissingClose;

#[node_impl]
impl MissingClose {
    fn init_device(&mut self) {}
}

fn main() {}
