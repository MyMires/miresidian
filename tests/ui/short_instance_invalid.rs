#[miresidian::node("缺少关联", instance(multiple))]
struct MissingBinding;
#[miresidian::node("单实例关联", instance(single, config))]
struct SingleBinding;
#[miresidian::node("重复策略", instance(parallel, selectable(sequential)))]
struct DuplicateExecution;
#[miresidian::node("缺少默认", instance(selectable()))]
struct MissingDefault;
#[miresidian::node("混用语法", instance(multiple, binding = config))]
struct MixedSyntax;
#[miresidian::node("重复标记", instance, instance())]
struct DuplicateFeature;
fn main() {}
