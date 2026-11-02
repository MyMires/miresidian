//! 参数级整数批量调用的无状态支持。
//!
//! 该 crate 不依赖节点运行时或过程宏。宏负责把已解析的数组长度和每次标量调用输出交给本 crate；
//! 根 `miresidian` crate 的 `batch` feature 负责将它接到节点 operation 包装层。

use miresidian_codec::{DeviceOutput, InvocationOutput, MiresidianError, MiresidianResult};

/// 校验批量整数数组的逐位置调用长度；长度为一的数组会广播到最长数组。
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

/// 将每次标量调用同位置的输出合并为可逆的竖线数组文本。
pub fn batch_outputs(outputs: Vec<InvocationOutput>) -> MiresidianResult<InvocationOutput> {
    let width = outputs.first().map_or(0, |output| output.as_slice().len());
    if outputs
        .iter()
        .any(|output| output.as_slice().len() != width)
    {
        return Err(MiresidianError::Internal(
            "批量调用返回了不一致的输出槽位数量".to_owned(),
        ));
    }
    let mut grouped = (0..width)
        .map(|_| Vec::with_capacity(outputs.len()))
        .collect::<Vec<_>>();
    for output in outputs {
        for (index, value) in output.into_values().into_iter().enumerate() {
            grouped[index].push(value);
        }
    }
    let mut values = Vec::with_capacity(width);
    for group in grouped {
        let encoded = <Vec<String> as DeviceOutput>::into_device_output(group)?;
        values.push(
            encoded
                .into_values()
                .into_iter()
                .next()
                .expect("Vec 输出总是一个平台文本值"),
        );
    }
    Ok(InvocationOutput::new(values))
}

#[cfg(test)]
mod tests {
    use super::{batch_length, batch_outputs};
    use miresidian_codec::{InvocationOutput, MiresidianError};

    #[test]
    fn single_values_broadcast_and_other_lengths_must_align() {
        assert_eq!(batch_length(&[("站号", 1), ("通道", 4)]).unwrap(), 4);
        assert!(matches!(
            batch_length(&[("站号", 2), ("通道", 4)]),
            Err(MiresidianError::InvalidArgument { field, .. }) if field == "站号"
        ));
    }

    #[test]
    fn outputs_group_each_slot_in_input_order() {
        assert_eq!(
            batch_outputs(vec![
                InvocationOutput::new(vec!["1".to_owned(), "A".to_owned()]),
                InvocationOutput::new(vec!["2".to_owned(), "B".to_owned()]),
            ])
            .unwrap()
            .as_slice(),
            ["1|2", "A|B"]
        );
    }
}
