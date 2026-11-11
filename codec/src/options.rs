//! 平台无关的枚举候选及规范值映射。

use crate::MiresidianResult;

/// 枚举的稳定实际值与显示文字；平台语法不属于此结构。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumOption {
    pub value: &'static str,
    pub display: &'static str,
}

/// 将具体显示值映射为实际值；标准集合按转义边界逐项处理。
///
/// 不选择首项，不处理平台参数名前缀或候选描述语法；未知文本保留给目标类型校验。
pub fn normalize_enum_text(
    value: &str,
    options: &[EnumOption],
    field: &str,
) -> MiresidianResult<String> {
    let normalize = |item: &str| {
        options
            .iter()
            .find(|option| item.trim() == option.value || item.trim() == option.display)
            .map_or_else(|| item.to_owned(), |option| option.value.to_owned())
    };
    if value.contains('|') {
        let items = super::collection::split_items(value, field)?;
        Ok(super::collection::format_items(
            items.iter().map(|item| normalize(item)),
        ))
    } else {
        Ok(normalize(value))
    }
}
