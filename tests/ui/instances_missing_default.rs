use miresidian::node;
#[node("非法实例策略", instances(mode = multiple, binding = config, execution = selectable))]
struct Invalid;
fn main() {}
