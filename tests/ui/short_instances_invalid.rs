#[miresidian::node("缺少关联", instances(multiple))]
struct MissingBinding;
#[miresidian::node("单实例关联", instances(single, config))]
struct SingleBinding;
#[miresidian::node("重复策略", instances(parallel, selectable(sequential)))]
struct DuplicateExecution;
#[miresidian::node("缺少默认", instances(selectable()))]
struct MissingDefault;
#[miresidian::node("混用语法", instances(multiple, binding = config))]
struct MixedSyntax;
#[miresidian::node("重复标记", instances, instances())]
struct DuplicateFeature;
fn main() {}
