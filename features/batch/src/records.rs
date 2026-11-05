//! 按原类型解析集合，或只拆分不透明的编码记录。
use miresidian_codec::{DeviceCollectionElement, DeviceInput, MiresidianResult};

/// 使用原方法参数类型的集合契约解析批量输入。
///
/// 整数保留单值、列表、递增范围及混合形式；文本、布尔等使用各自的元素解析。
/// 空文本是一条空记录，由 T 决定是否合法，不改变普通 Vec 的空集合语义。
/// `field` 用于标识非法元素、范围或转义的错误来源；解析失败不返回部分结果。
pub fn decode_batch<T: DeviceInput + DeviceCollectionElement>(
    value: &str,
    field: &str,
) -> MiresidianResult<Vec<T>> {
    if value.is_empty() {
        T::parse_device_text(value, field).map(|value| vec![value])
    } else {
        T::parse_device_collection(value, field)
    }
}

/// 拆分显式编码等不透明记录；只处理外层竖线与转义，不解释类型或编码。
///
/// 空文本表示一条空记录。调用方可以独立地把记录交给格式转换或其他业务解析器。
/// 非法或未完成的反斜杠转义返回带 `field` 的参数错误。
pub fn batch_records(value: &str, field: &str) -> MiresidianResult<Vec<String>> {
    if value.is_empty() {
        Ok(vec![String::new()])
    } else {
        Vec::<String>::parse_device_text(value, field)
    }
}

#[cfg(test)]
mod tests {
    use super::{batch_records, decode_batch};
    use miresidian_codec::DeviceInput;
    #[test]
    fn typed_input_and_opaque_records_have_distinct_grammars() {
        assert_eq!(
            decode_batch::<u16>("1-3|5|8-10", "值").unwrap(),
            [1, 2, 3, 5, 8, 9, 10]
        );
        assert_eq!(decode_batch::<String>("1-3|5", "值").unwrap(), ["1-3", "5"]);
        assert_eq!(
            decode_batch::<bool>("true|0|是", "值").unwrap(),
            [true, false, true]
        );
        assert_eq!(decode_batch::<String>("", "值").unwrap(), [""]);
        assert!(decode_batch::<bool>("", "值").is_err());
        assert_eq!(decode_batch::<i32>("", "值").unwrap(), [0]);
        assert!(
            Vec::<String>::parse_device_text("", "值")
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            batch_records(r"1-3|A\|B|C\\D", "值").unwrap(),
            ["1-3", "A|B", r"C\D"]
        );
        assert_eq!(batch_records("", "值").unwrap(), [""]);
    }
}
