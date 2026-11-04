//! 平台参数名称、默认值与实际值的统一提取规则。

use crate::MiresidianResult;

use super::invalid;

/// 从 Config 或 Input 数组中取出一个参数的实际文本。
///
/// 调用方可传裸值，也可传入 `名称:值`。提供项为空白且属性定义了默认值时使用默认值；未提供
/// 且没有默认值时报告缺失。保留非空裸字符串的原始空白，避免文本数据在框架层被意外修改。
pub(super) fn value_at(
    values: &[String],
    index: usize,
    field: &str,
    default: Option<&str>,
) -> MiresidianResult<String> {
    let Some(raw) = values.get(index) else {
        return default
            .map(str::to_owned)
            .ok_or_else(|| invalid(field, "缺少参数值"));
    };

    let value = remove_name_prefix(raw, field);
    if value.trim().is_empty()
        && let Some(default) = default
    {
        return Ok(default.to_owned());
    }
    Ok(value.to_owned())
}

/// 解析匹配声明的默认候选文本，并将“实际值-显示值”映射为稳定的实际值。
///
/// 标量直接匹配；包含 `|` 的数组文本会逐项映射，随后交给既有集合 codec 继续处理。
pub(super) fn normalize_option_value(value: String, definition: &str) -> String {
    // 平台可直接传入整段默认候选文本；只有与声明完全匹配时才取首项。
    let value = declared_choice_value(&value, definition)
        .map(str::to_owned)
        .unwrap_or(value);
    let options = option_pairs(definition);
    if options.is_empty() {
        return value;
    }
    if value.contains('|') {
        return value
            .split('|')
            .map(|item| normalize_option_item(item, &options))
            .collect::<Vec<_>>()
            .join("|");
    }
    normalize_option_item(&value, &options)
}

/// 识别整段候选文本，不将任意显式输入回退到首项，也不改写普通路径默认值。
fn declared_choice_value<'a>(value: &str, definition: &'a str) -> Option<&'a str> {
    let (_, suffix) = definition.split_once(':')?;
    let suffix = suffix.trim();
    if value.trim() != suffix {
        return None;
    }
    let mut choices = suffix.split('/').map(str::trim);
    let first = choices.next()?;
    if choices.clone().next().is_none()
        || first.is_empty()
        || matches!(first, "." | "..")
        || first.contains(':')
        || choices.any(str::is_empty)
    {
        return None;
    }
    // 负数及科学计数法中的连字符属于数值，不是实际值/显示值分隔符。
    if first.parse::<f64>().is_ok() {
        return Some(first);
    }
    match first.split_once('-') {
        Some((actual, display)) if !actual.trim().is_empty() && !display.trim().is_empty() => {
            Some(actual.trim())
        }
        _ => Some(first),
    }
}

fn normalize_option_item(value: &str, options: &[(&str, &str)]) -> String {
    let trimmed = value.trim();
    options
        .iter()
        .find(|(actual, display)| trimmed == *actual || trimmed == *display)
        .map(|(actual, _)| (*actual).to_owned())
        .unwrap_or_else(|| value.to_owned())
}

fn option_pairs(definition: &str) -> Vec<(&str, &str)> {
    let Some((_, choices)) = definition.split_once(':') else {
        return Vec::new();
    };
    choices
        .split('/')
        .filter_map(|choice| {
            let (actual, display) = choice.split_once('-')?;
            let actual = actual.trim();
            let display = display.trim();
            (!actual.is_empty() && !display.is_empty()).then_some((actual, display))
        })
        .collect()
}

fn remove_name_prefix<'a>(value: &'a str, field: &str) -> &'a str {
    let leading_trimmed = value.trim_start();
    let Some(rest) = leading_trimmed.strip_prefix(field) else {
        return value;
    };
    let Some(rest) = rest.strip_prefix(':') else {
        return value;
    };
    rest.trim_start()
}
