//! 宏组合层：记录拆分和对齐属于 batch，单条选择及解码属于 format。
use super::format::Format;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Type};

/// 有选择器时解析一列格式；固定单编码只有一个常量，不占用 Input。
pub(super) fn selection(format: &Format, index: usize) -> TokenStream {
    let formats = format.formats();
    if let Some(name) = &format.selector {
        quote!({
            let text = ::miresidian::codec::normalize_value_with_definition(input, #index, #name, #name, Some(""))?;
            ::miresidian::__private::batch_records(&text, #name)?.iter()
                .map(|record| ::miresidian::format::select_format_value(record, #name, #formats))
                .collect::<::miresidian::MiresidianResult<::std::vec::Vec<_>>>()?
        })
    } else {
        quote!(::std::vec![(#formats)[0]])
    }
}

/// 先对齐编码记录与格式列，再逐条调用单条格式解码；不重解释解码结果中的分隔符。
pub(super) fn decode(ty: &Type, name: &LitStr, format: &Format, selected: &Ident) -> TokenStream {
    let selector = format.selector.as_ref().unwrap_or(name);
    quote!({
        let records = ::miresidian::__private::batch_records(&text, #name)?;
        let length = ::miresidian::__private::batch_length(&[(#name, records.len()), (#selector, #selected.len())])?;
        let mut decoded = ::std::vec::Vec::with_capacity(length);
        for index in 0..length {
            decoded.push(::miresidian::format::decode_formatted::<#ty>(
                &records[if records.len() == 1 { 0 } else { index }],
                #selected[if #selected.len() == 1 { 0 } else { index }], #name,
            )?);
        }
        decoded
    })
}
