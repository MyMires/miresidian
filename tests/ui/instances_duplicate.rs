use miresidian::node;
#[node("非法实例策略", instances(mode = multiple, mode = single, execution = sequential))]
struct Invalid;
fn main() {}
