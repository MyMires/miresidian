use miresidian::node;
#[node("非法实例策略", instance(mode = single, execution = selectable, default_execution = selectable))]
struct Invalid;
fn main() {}
