use miresidian_macro::node_impl;

struct GenericNode<T>(T);

#[node_impl]
impl<T> GenericNode<T> {}

fn main() {}
