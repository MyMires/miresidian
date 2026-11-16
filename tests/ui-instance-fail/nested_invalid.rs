#[miresidian::node("空关联", instance(multiple()))]
struct Empty;
#[miresidian::node("未知关联", instance(multiple(other)))]
struct Unknown;
#[miresidian::node("双关联", instance(multiple(config, handle)))]
struct Two;
#[miresidian::node("重复关联", instance(multiple(config), config))]
struct Duplicate;
#[miresidian::node("冲突关联", instance(handle, multiple(config)))]
struct Conflict;
#[miresidian::node("重复模式", instance(multiple, multiple(handle)))]
struct Mode;
#[miresidian::node("单实例关联", instance(single(config)))]
struct Single;
fn main() {}
