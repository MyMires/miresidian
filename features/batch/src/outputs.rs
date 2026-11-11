//! 将逐次调用结果按输出槽位聚合，复用可逆集合编码。
use miresidian_codec::{InvocationOutput, MiresidianError, MiresidianResult, NodeOutput};

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
        let encoded = <Vec<String> as NodeOutput>::into_output(group)?;
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
    use super::batch_outputs;
    use miresidian_codec::InvocationOutput;
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
