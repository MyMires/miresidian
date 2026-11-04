//! 根框架数据编解码的跨模块回归测试。

use miresidian_codec::{
    ByteTextFormat, DeviceInput, DeviceOutput, MiresidianError, decode_text_to_bytes, decode_value,
    decode_value_with_definition, encode_bytes_to_text, transcode_text,
};

#[test]
fn utf8_平台选项兼容新旧名称() {
    for name in ["UTF_8", "utf_8", "UTF8", "UTF-8"] {
        assert_eq!(
            name.parse::<ByteTextFormat>().unwrap(),
            ByteTextFormat::Utf8
        );
        let values = vec![name.to_owned()];
        let format = decode_value_with_definition::<ByteTextFormat>(
            &values,
            0,
            "数据格式",
            "数据格式:ASCII/GBK/UTF_8/Unicode/HEX/Base64",
            None,
        )
        .unwrap();
        assert_eq!(
            decode_text_to_bytes("测试", format).unwrap(),
            "测试".as_bytes()
        );
        assert_eq!(
            encode_bytes_to_text("测试".as_bytes(), format).unwrap(),
            "测试"
        );
    }
}

#[test]
fn 文本格式可以经由原始字节无损转码() {
    assert_eq!(
        transcode_text("测试", ByteTextFormat::Gbk, ByteTextFormat::Hex).expect("GBK 应可转 HEX"),
        "B2 E2 CA D4"
    );
    assert_eq!(
        transcode_text("B2 E2 CA D4", ByteTextFormat::Hex, ByteTextFormat::Utf8)
            .expect_err("GBK 字节不能被当作 UTF-8")
            .to_string(),
        "参数 数据 无效: 字节序列不是有效 UTF-8"
    );
    assert_eq!(
        transcode_text("B2 E2 CA D4", ByteTextFormat::Hex, ByteTextFormat::Gbk)
            .expect("HEX 应可转 GBK"),
        "测试"
    );
    assert_eq!(
        transcode_text("测试", ByteTextFormat::Utf16Le, ByteTextFormat::Hex)
            .expect("Unicode 应可转 HEX"),
        "4B 6D D5 8B"
    );
}

#[test]
fn ascii_是_gbk_的严格子集() {
    assert_eq!(
        transcode_text("ABC", ByteTextFormat::Ascii, ByteTextFormat::Gbk)
            .expect("ASCII 应兼容 GBK"),
        "ABC"
    );
    assert!(decode_text_to_bytes("测试", ByteTextFormat::Ascii).is_err());
    assert!(encode_bytes_to_text(&[0xB2, 0xE2], ByteTextFormat::Ascii).is_err());
}

#[test]
fn hex_与_base64_采用稳定字节规范() {
    let bytes = decode_text_to_bytes("0x00 FF 0A", ByteTextFormat::Hex).expect("HEX 输入应有效");
    assert_eq!(bytes, vec![0, 255, 10]);
    assert_eq!(
        encode_bytes_to_text(&bytes, ByteTextFormat::Hex).expect("HEX 输出应有效"),
        "00 FF 0A"
    );
    assert_eq!(
        encode_bytes_to_text(&bytes, ByteTextFormat::Base64).expect("Base64 输出应有效"),
        "base64:AP8K"
    );
    assert_eq!(
        decode_text_to_bytes("base64:AP8K", ByteTextFormat::Base64).expect("Base64 输入应有效"),
        bytes
    );
}

#[test]
fn 标量与数组规则可由宏生成调用复用() {
    let values = vec!["数值:0xFF".to_owned(), "0b1010".to_owned()];
    assert_eq!(
        decode_value::<u16>(&values, 0, "数值", None).expect("十六进制数值"),
        255
    );
    assert_eq!(
        decode_value::<u16>(&values, 1, "位数", None).expect("二进制数值"),
        10
    );

    let array = Vec::<String>::parse_device_text("A\\|B|C\\\\D", "项目").expect("可逆数组");
    assert_eq!(array, vec!["A|B", "C\\D"]);
    assert_eq!(
        array.into_device_output().expect("数组输出").as_slice(),
        ["A\\|B|C\\\\D"]
    );

    let range = Vec::<u16>::parse_device_text("1-3|0x10", "通道").expect("整数范围");
    assert_eq!(range, vec![1, 2, 3, 16]);
}

#[test]
fn 字节数组始终以_base64_输出() {
    let bytes = Vec::<u8>::parse_device_text("base64:AP8K", "数据").expect("Base64 字节数组");
    assert_eq!(bytes, vec![0, 255, 10]);
    assert_eq!(
        bytes.into_device_output().expect("字节输出").as_slice(),
        ["base64:AP8K"]
    );
}

#[test]
fn 选项显示值会在标量与数组解析前规范化为实际值() {
    let scalar = vec!["奇校验".to_owned()];
    assert_eq!(
        decode_value_with_definition::<String>(
            &scalar,
            0,
            "校验位",
            "校验位:None-无校验/Odd-奇校验/Even-偶校验",
            None,
        )
        .expect("显示值应映射为实际值"),
        "Odd"
    );

    let array = vec!["无校验|偶校验".to_owned()];
    assert_eq!(
        decode_value_with_definition::<Vec<String>>(
            &array,
            0,
            "校验位",
            "校验位:None-无校验/Odd-奇校验/Even-偶校验",
            None,
        )
        .expect("数组显示值应逐项映射"),
        vec!["None", "Even"]
    );
}

#[test]
fn 默认候选文本只在采用该文本时取首项() {
    let definition = "波特率:9600/19200";
    for values in [
        vec![],
        vec!["".to_owned()],
        vec!["波特率:".to_owned()],
        vec!["9600/19200".to_owned()],
    ] {
        assert_eq!(
            decode_value_with_definition::<u32>(
                &values,
                0,
                "波特率",
                definition,
                Some("9600/19200")
            )
            .unwrap(),
            9600
        );
    }
    for value in ["19200", "波特率:19200", "38400"] {
        assert_eq!(
            decode_value_with_definition::<u32>(
                &[value.to_owned()],
                0,
                "波特率",
                definition,
                Some("9600/19200")
            )
            .unwrap(),
            if value == "38400" { 38400 } else { 19200 }
        );
    }
    for value in ["bad", "19200/38400"] {
        let error = decode_value_with_definition::<u32>(
            &[value.to_owned()],
            0,
            "波特率",
            definition,
            Some("9600/19200"),
        )
        .unwrap_err();
        assert!(
            matches!(error, MiresidianError::InvalidArgument { field, .. } if field == "波特率")
        );
    }
}

#[test]
fn 选项实际值与显示值不被默认候选覆盖() {
    let definition = "校验:None-无校验/Odd-奇校验/Even-偶校验";
    for (value, expected) in [
        ("None-无校验/Odd-奇校验/Even-偶校验", "None"),
        ("Odd", "Odd"),
        ("奇校验", "Odd"),
        ("Even", "Even"),
        ("偶校验", "Even"),
        ("其他", "其他"),
    ] {
        assert_eq!(
            decode_value_with_definition::<String>(
                &[value.to_owned()],
                0,
                "校验",
                definition,
                None
            )
            .unwrap(),
            expected
        );
    }
    assert!(decode_value_with_definition::<String>(&[], 0, "校验", definition, None).is_err());
    assert_eq!(
        decode_value_with_definition::<String>(&[], 0, "校验", definition, Some("Odd")).unwrap(),
        "Odd"
    );
}

#[test]
fn 普通默认文本与显式文本保持原值() {
    for value in [
        "https://example.test/a/b",
        "/tmp/device/config",
        "./config/device",
        "../config/device",
        "C:/device/config",
        "-1",
    ] {
        let definition = format!("值:{value}");
        assert_eq!(
            decode_value_with_definition::<String>(&[], 0, "值", &definition, Some(value)).unwrap(),
            value
        );
        assert_eq!(
            decode_value_with_definition::<String>(
                &[value.to_owned()],
                0,
                "值",
                &definition,
                Some(value)
            )
            .unwrap(),
            value
        );
    }
    assert_eq!(
        decode_value_with_definition::<i32>(&[], 0, "偏移", "偏移:-1/0/1", Some("-1/0/1")).unwrap(),
        -1
    );
}
