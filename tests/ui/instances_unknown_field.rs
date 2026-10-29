use miresidian::node;
#[node("非法实例策略", instances(mode = multiple, binding = config, execution = sequential, unknown = single))]
struct Invalid;
fn main() {}
