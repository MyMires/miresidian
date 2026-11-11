#[miresidian::node("重复租约", lease, lease(required))]
struct Duplicate;
#[miresidian::node("缺少默认", lease(selectable))]
struct MissingDefault;
#[miresidian::node("非法默认", lease(selectable(disabled)))]
struct InvalidDefault;
#[miresidian::node("混用字段", lease(required, activation = required))]
struct MixedSyntax;
fn main() {}
