#[miresidian::node("禁用节点", instances(mode = multiple, binding = handle, execution = parallel))]
#[cfg(any())]
struct Ignored;
fn main() {}
