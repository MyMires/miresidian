//! 批次各列按位置对齐及单值广播。
use miresidian_codec::{MiresidianError, MiresidianResult};

/// 校验记录列的逐位置调用长度；长度为一的列会广播到最长列。
pub fn batch_length(parameters: &[(&str, usize)]) -> MiresidianResult<usize> {
    let length = parameters
        .iter()
        .map(|(_, length)| *length)
        .max()
        .unwrap_or(0);
    if length == 0 {
        return Err(MiresidianError::InvalidArgument {
            field: "batch".to_owned(),
            message: "批量参数不能为空".to_owned(),
        });
    }
    if let Some((name, actual)) = parameters
        .iter()
        .find(|(_, actual)| *actual != 1 && *actual != length)
    {
        return Err(MiresidianError::InvalidArgument {
            field: (*name).to_owned(),
            message: format!("批量参数长度 {actual} 无法与最大长度 {length} 按位置对齐"),
        });
    }
    Ok(length)
}

#[cfg(test)]
mod tests {
    use super::batch_length;
    use miresidian_codec::MiresidianError;
    #[test]
    fn single_values_broadcast_and_other_lengths_must_align() {
        assert_eq!(batch_length(&[("站号", 1), ("通道", 4)]).unwrap(), 4);
        assert!(matches!(
            batch_length(&[("站号", 2), ("通道", 4)]),
            Err(MiresidianError::InvalidArgument { field, .. }) if field == "站号"
        ));
    }
}
