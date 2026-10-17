use miresidian_macros::node_impl;

struct MissingInit;

#[node_impl]
impl MissingInit {
    fn close_device(&mut self) {}
}

fn main() {}
