use miresidian_codec::ByteTextFormat::*;
use miresidian_format::{
    decode_formatted, decode_formatted_batch, encode_formatted, select_format, select_format_value,
};

#[test]
fn single_format_selection_does_not_parse_batch_columns() {
    assert_eq!(select_format_value("", "格式", &[Utf8, Hex]).unwrap(), Utf8);
    assert_eq!(
        select_format_value("HEX", "格式", &[Utf8, Hex]).unwrap(),
        Hex
    );
    assert_eq!(select_format(&[], 0, "格式", &[Utf8, Hex]).unwrap(), Utf8);
    for value in ["UTF8|HEX", "Base64", "unknown"] {
        assert!(
            matches!(select_format_value(value, "格式", &[Utf8, Hex]), Err(miresidian_codec::MiresidianError::InvalidArgument { field, .. }) if field == "格式")
        );
    }
    assert!(select_format_value("UTF8", "格式", &[]).is_err());
}
#[test]
fn raw_bytes_all_formats_round_trip() {
    for format in [Ascii, Gbk, Utf8, Utf16Le, Hex, Base64] {
        let bytes = miresidian_codec::decode_text_to_bytes("AB", format)
            .unwrap_or_else(|_| vec![0x41, 0x42]);
        let encoded = encode_formatted(bytes.clone(), format).unwrap();
        assert_eq!(
            decode_formatted::<Vec<u8>>(&encoded, format, "数据").unwrap(),
            bytes
        );
    }
    assert!(decode_formatted::<Vec<u8>>("GG", Hex, "数据").is_err());
    assert!(encode_formatted(vec![0xff_u8], Utf8).is_err());
}
#[test]
fn typed_text_and_sequences_do_not_use_binary_memory_layout() {
    assert_eq!(decode_formatted::<i32>("31 32", Hex, "数值").unwrap(), 12);
    assert_eq!(encode_formatted(12_i32, Hex).unwrap(), "31 32");
    assert_eq!(
        decode_formatted::<Vec<f64>>("3.125 3.15", Utf8, "数组").unwrap(),
        vec![3.125, 3.15]
    );
    assert_eq!(encode_formatted(vec![1_u16, 2], Utf8).unwrap(), "1 2");
    assert!(decode_formatted::<bool>("1", Utf8, "布尔").unwrap());
    assert_eq!(encode_formatted(true, Utf8).unwrap(), "true");
    for format in [Gbk, Utf16Le, Utf8] {
        let encoded = encode_formatted("中文".to_string(), format).unwrap();
        assert_eq!(
            decode_formatted::<String>(&encoded, format, "文本").unwrap(),
            "中文"
        );
    }
    assert!(encode_formatted("中文".to_string(), Ascii).is_err());
    assert!(decode_formatted::<u8>("256", Utf8, "数值").is_err());
    assert!(decode_formatted::<f64>("1-5", Utf8, "数值").is_err());
}
#[test]
fn optional_and_empty_batch_records_preserve_positions() {
    assert_eq!(
        decode_formatted::<Option<Vec<u8>>>("", Hex, "值").unwrap(),
        None
    );
    assert_eq!(
        decode_formatted_batch::<Vec<u8>>("", Hex, "值").unwrap(),
        vec![Vec::<u8>::new()]
    );
    assert_eq!(
        decode_formatted_batch::<Vec<u8>>("|41", Hex, "值").unwrap(),
        vec![vec![], vec![0x41]]
    );
}

#[test]
fn nonfinite_float_outputs_fail_for_all_formats_and_wrappers() {
    for format in [Ascii, Gbk, Utf8, Utf16Le, Hex, Base64] {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                encode_formatted(value, format).unwrap_err().code(),
                "INVALID_ARGUMENT"
            );
            assert!(encode_formatted(vec![1.0, value], format).is_err());
            assert!(encode_formatted(Some(value), format).is_err());
            assert!(encode_formatted(Some(vec![value]), format).is_err());
            assert!(encode_formatted(value as f32, format).is_err());
            assert!(encode_formatted(vec![value as f32], format).is_err());
        }
        let encoded = encode_formatted(1.25_f64, format).unwrap();
        assert_eq!(
            decode_formatted::<f64>(&encoded, format, "数值").unwrap(),
            1.25
        );
        assert_eq!(
            encode_formatted(None::<f64>, format).unwrap(),
            if format == Base64 { "base64:" } else { "" }
        );
    }
    assert!(encode_formatted(vec![0xff, 0xfe, 0x41, 0_u8], Gbk).is_err());
    assert_eq!(decode_formatted::<f64>("", Utf8, "数值").unwrap(), 0.0);
}
