use miresidian::node;
#[node("非法实例策略", instances(mode = multiple, binding = config, execution = parallel, default_execution = sequential))]
struct Invalid;
fn main() {}
