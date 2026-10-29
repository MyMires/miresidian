use miresidian::node;
#[node("非法实例策略", instances(mode = multiple, execution = sequential))]
struct Invalid;
fn main() {}
