use miresidian::node;
#[node("非法实例策略", instances(mode = single, binding = config, execution = sequential))]
struct Invalid;
fn main() {}
