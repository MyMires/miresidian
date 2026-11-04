#![cfg(feature = "format")]
//! 复用同一节点契约，经 Runtime、原生 async 和 C ABI 验证格式列、批量及输出槽位。
use miresidian::{Invocation, MiresidianResult, NodeRuntime};

#[miresidian::node("格式转换")]
struct FormatNode {
    calls: usize,
}
impl FormatNode {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        Ok(Self { calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("转换")]
    #[miresidian::output("数据", format(formats = [hex, utf8], selector = "输出格式"))]
    #[miresidian::output("次数")]
    fn convert(
        &mut self,
        #[miresidian::input("数据", format(formats = [utf8, hex], selector = "输入格式"))]
        data: Vec<u8>,
        #[miresidian::input("后缀:!")] suffix: String,
    ) -> MiresidianResult<(Vec<u8>, usize)> {
        self.calls += 1;
        let mut data = data;
        data.extend(suffix.as_bytes());
        Ok((data, self.calls))
    }
    #[miresidian::operation("异步")]
    #[miresidian::output("数据", format(formats = [hex]))]
    async fn echo_async(
        &mut self,
        #[miresidian::input("数据", format(formats = [hex]))] data: Vec<u8>,
    ) -> MiresidianResult<Vec<u8>> {
        Ok(data)
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("批量")]
    #[miresidian::output("数据", format(formats = [hex, utf8], selector = "输出格式"))]
    #[miresidian::output("次数")]
    fn batch(
        &mut self,
        #[miresidian::input("数据", batch, format(formats = [hex, utf8], selector = "输入格式"))]
        data: Vec<u8>,
        #[miresidian::input("倍率", batch)] scale: f64,
    ) -> MiresidianResult<(Vec<u8>, usize)> {
        assert!(scale > 0.0);
        self.calls += 1;
        Ok((data, self.calls))
    }
    #[cfg(feature = "batch")]
    #[miresidian::operation("顺序失败")]
    #[miresidian::output("数据", format(formats = [utf8]))]
    fn fail(&mut self, #[miresidian::input("序号", batch)] n: u16) -> MiresidianResult<Vec<u8>> {
        self.calls += 1;
        Ok(if n == 2 { vec![0xff] } else { vec![b'A'] })
    }
    #[miresidian::output("数据", format(formats = [hex]))]
    #[miresidian::operation("单元组")]
    fn single_tuple(&mut self) -> MiresidianResult<(Vec<u8>,)> {
        Ok((vec![0x41],))
    }

    #[miresidian::operation("次数")]
    fn count(&mut self) -> MiresidianResult<usize> {
        Ok(self.calls)
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("逐条格式")]
    #[miresidian::output("文本", format(formats = [utf8, hex, base64], selector = "输出类型"))]
    #[miresidian::output("开关")]
    fn per_record(
        &mut self,
        #[miresidian::input("文本", batch, format(formats = [utf8, hex, base64], selector = "输入类型"))]
        text: String,
        #[miresidian::input("开关", batch)] flag: bool,
    ) -> MiresidianResult<(String, bool)> {
        self.calls += 1;
        Ok((text, flag))
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("固定文本编码")]
    #[miresidian::output("文本")]
    fn fixed_text(
        &mut self,
        #[miresidian::input("文本", batch, format(formats = [hex]))] text: String,
    ) -> MiresidianResult<String> {
        self.calls += 1;
        Ok(text)
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("多列组合")]
    #[miresidian::output("文本", format(formats = [utf8, hex], selector = "返回类型"))]
    #[miresidian::output("开关")]
    #[miresidian::output("序号")]
    fn multiple_columns(
        &mut self,
        #[miresidian::input("文本", batch, format(formats = [utf8, hex, base64], selector = "文本类型"))]
        text: String,
        #[miresidian::input("开关", batch, format(formats = [utf8, hex, base64], selector = "开关类型"))]
        flag: bool,
        #[miresidian::input("序号", batch)] index: u16,
        #[miresidian::input("后缀:!")] suffix: String,
    ) -> MiresidianResult<(String, bool, u16)> {
        self.calls += 1;
        Ok((text + &suffix, flag, index))
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("异步批量格式")]
    async fn batch_async(
        &mut self,
        #[miresidian::input("文本", batch, format(formats = [hex]))] text: String,
        #[miresidian::input("开关", batch)] flag: bool,
    ) -> MiresidianResult<(String, bool)> {
        std::future::ready(()).await;
        self.calls += 1;
        Ok((text, flag))
    }
}
fn runtime() -> NodeRuntime<FormatNode> {
    let runtime = NodeRuntime::new();
    runtime
        .invoke(&Invocation::new("InitDevice", vec![], vec![]))
        .unwrap();
    runtime
}
fn invoke(
    runtime: &NodeRuntime<FormatNode>,
    name: &str,
    input: &[&str],
) -> miresidian::MiresidianResult<miresidian::InvocationOutput> {
    runtime.invoke(&Invocation::new(
        name,
        vec![],
        input.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    ))
}
#[test]
fn descriptors_defaults_and_independent_selectors() {
    let runtime = runtime();
    let functions = invoke(&runtime, "GetFunctions", &[]).unwrap();
    assert!(functions.as_slice().iter().any(|s| s.contains("转换(数据,输入格式:UTF8/HEX,后缀:!,输出格式:HEX/UTF8)(数据,次数)")), "{functions:?}");
    assert_eq!(
        invoke(&runtime, "转换", &["31 32", "HEX", "!", "HEX"])
            .unwrap()
            .as_slice(),
        ["31 32 21", "1"]
    );
    assert_eq!(
        invoke(&runtime, "转换", &["AB", "", "", ""])
            .unwrap()
            .as_slice(),
        ["41 42 21", "2"]
    );
    assert_eq!(
        invoke(&runtime, "转换", &["AB", "", ""])
            .unwrap()
            .as_slice(),
        ["41 42 21", "3"]
    );
    assert!(invoke(&runtime, "转换", &["AB", "UTF8", "!", "Base64"]).is_err());
    assert!(invoke(&runtime, "转换", &["ZZ", "HEX", "!", "HEX"]).is_err());
    assert!(invoke(&runtime, "转换", &["AB", "UTF8|HEX", "!", "UTF8"]).is_err());
    assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["3"]);
}
#[test]
fn fixed_format_async_has_no_selector() {
    let runtime = runtime();
    assert_eq!(invoke(&runtime, "单元组", &[]).unwrap().as_slice(), ["41"]);
    assert_eq!(
        invoke(&runtime, "异步", &["31 32"]).unwrap().as_slice(),
        ["31 32"]
    );
}
#[cfg(feature = "batch")]
#[test]
fn batch_bytes_floats_empty_records_and_escaping() {
    let runtime = runtime();
    assert_eq!(
        invoke(
            &runtime,
            "批量",
            &["31 32|44 45", "HEX", "3.14|3.15", "HEX"]
        )
        .unwrap()
        .as_slice(),
        ["31 32|44 45", "1|2"]
    );
    assert_eq!(
        invoke(&runtime, "批量", &["|41", "HEX", "1", "HEX"])
            .unwrap()
            .as_slice(),
        ["|41", "3|4"]
    );
    assert_eq!(
        invoke(&runtime, "批量", &[r"A\|B|C\\D", "UTF8", "1", "UTF8"])
            .unwrap()
            .as_slice(),
        [r"A\|B|C\\D", "5|6"]
    );
    assert!(invoke(&runtime, "批量", &["41|42", "HEX", "1-5", "HEX"]).is_err());
    assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["6"]);
}
#[cfg(feature = "batch")]
#[test]
fn output_conversion_error_stops_subsequent_records() {
    let runtime = runtime();
    assert!(invoke(&runtime, "顺序失败", &["1-3"]).is_err());
    assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["2"]);
}

#[cfg(feature = "batch")]
#[test]
fn format_columns_broadcast_and_align_with_typed_batch_inputs() {
    let runtime = runtime();
    assert_eq!(
        invoke(
            &runtime,
            "逐条格式",
            &["A|42|Qw==", "UTF8|HEX|Base64", "true", "UTF8"]
        )
        .unwrap()
        .as_slice(),
        ["A|B|C", "true|true|true"]
    );
    assert_eq!(
        invoke(
            &runtime,
            "逐条格式",
            &["41", "UTF8|HEX", "true|false", "UTF8"]
        )
        .unwrap()
        .as_slice(),
        ["41|A", "true|false"]
    );
    assert_eq!(
        invoke(
            &runtime,
            "逐条格式",
            &["A", "UTF8", "true", "UTF8|HEX|Base64"]
        )
        .unwrap()
        .as_slice(),
        ["A|41|base64:QQ==", "true|true|true"]
    );
    assert_eq!(
        invoke(
            &runtime,
            "逐条格式",
            &["A|42", "UTF8|HEX", "true", "HEX|UTF8"]
        )
        .unwrap()
        .as_slice(),
        ["41|B", "true|true"]
    );
    assert_eq!(
        invoke(
            &runtime,
            "逐条格式",
            &[r"A\|B|Qw==", "UTF8|Base64", "true", "UTF8"]
        )
        .unwrap()
        .as_slice(),
        [r"A\|B|C", "true|true"]
    );
    assert_eq!(
        invoke(
            &runtime,
            "逐条格式",
            &["A|B|Qw==", "UTF8||Base64", "true", ""]
        )
        .unwrap()
        .as_slice(),
        ["A|B|C", "true|true|true"]
    );
}

#[cfg(feature = "batch")]
#[test]
fn fixed_single_format_uses_no_selector_and_applies_to_every_record() {
    let runtime = runtime();
    let functions = invoke(&runtime, "GetFunctions", &[]).unwrap();
    assert!(
        functions
            .as_slice()
            .iter()
            .any(|s| s == "固定文本编码(文本)(文本)")
    );
    assert_eq!(
        invoke(&runtime, "固定文本编码", &["41|42"])
            .unwrap()
            .as_slice(),
        ["A|B"]
    );
    assert_eq!(
        invoke(&runtime, "固定文本编码", &[""]).unwrap().as_slice(),
        [""]
    );
}

#[cfg(feature = "batch")]
#[test]
fn format_list_errors_are_rejected_before_business_execution() {
    let runtime = runtime();
    for input in [
        vec!["A|B", "UTF8|HEX|Base64", "true", "UTF8"],
        vec!["A|B", "UTF8|unknown", "true", "UTF8"],
        vec!["A|B", "UTF8", "true", "UTF8|HEX|Base64"],
        vec!["A|B", "UTF8", "true|invalid", "UTF8"],
    ] {
        assert!(invoke(&runtime, "逐条格式", &input).is_err());
        assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["0"]);
    }
}

#[cfg(feature = "batch")]
#[test]
fn multiple_formats_mixed_integer_ranges_and_plain_defaults_keep_input_positions() {
    let runtime = runtime();
    let functions = invoke(&runtime, "GetFunctions", &[]).unwrap();
    assert!(functions.as_slice().iter().any(|s| s == "多列组合(文本,文本类型:UTF8/HEX/Base64,开关,开关类型:UTF8/HEX/Base64,序号,后缀:!,返回类型:UTF8/HEX)(文本,开关,序号)"));
    assert_eq!(
        invoke(
            &runtime,
            "多列组合",
            &[
                "A|42|Qw==",
                "UTF8|HEX|Base64",
                "true|66 61 6C 73 65|dHJ1ZQ==",
                "UTF8|HEX|Base64",
                "1-2|4",
                "",
                "UTF8|HEX|UTF8"
            ]
        )
        .unwrap()
        .as_slice(),
        ["A!|42 21|C!", "true|false|true", "1|2|4"]
    );
    assert_eq!(invoke(&runtime, "次数", &[]).unwrap().as_slice(), ["3"]);
}

#[cfg(feature = "batch")]
#[tokio::test]
async fn native_async_batch_keeps_fixed_format_and_bool_broadcast() {
    let runtime = NodeRuntime::<FormatNode>::new();
    runtime
        .invoke_async(Invocation::new("InitDevice", vec![], vec![]))
        .await
        .unwrap();
    let output = runtime
        .invoke_async(Invocation::new(
            "异步批量格式",
            vec![],
            vec!["41|42".into(), "true".into()],
        ))
        .await
        .unwrap();
    assert_eq!(output.as_slice(), ["A|B", "true|true"]);
}

#[cfg(feature = "batch")]
#[test]
fn c_abi_preserves_json_slots_and_prevalidation_for_typed_format_columns() {
    use std::ffi::{CStr, CString};
    let call = |operation: &str, inputs: &[&str]| -> serde_json::Value {
        let operation = CString::new(operation).unwrap();
        let inputs = CString::new(serde_json::to_string(inputs).unwrap()).unwrap();
        // SAFETY: 输入 C 字符串在调用期间存活，响应在同线程下次调用前立即复制解析。
        let response = unsafe {
            CStr::from_ptr(miresidian_invoke(
                operation.as_ptr(),
                c"[]".as_ptr(),
                inputs.as_ptr(),
            ))
            .to_str()
            .unwrap()
            .to_owned()
        };
        serde_json::from_str(&response).unwrap()
    };
    assert_eq!(call("CloseAllDevices", &[])["status"], 0);
    assert_eq!(call("InitDevice", &[])["status"], 0);
    let response = call(
        "逐条格式",
        &["A|42|Qw==", "UTF8|HEX|Base64", "true", "UTF8"],
    );
    assert_eq!(response["code"], "OK");
    assert_eq!(
        response["output"],
        serde_json::json!(["A|B|C", "true|true|true"])
    );
    assert_eq!(
        call("固定文本编码", &["41|42"])["output"],
        serde_json::json!(["A|B"])
    );
    assert_eq!(
        call("逐条格式", &["A", "unknown", "true", "UTF8"])["code"],
        "INVALID_ARGUMENT"
    );
    assert_eq!(call("次数", &[])["output"], serde_json::json!(["5"]));
    assert_eq!(call("CloseAllDevices", &[])["status"], 0);
}
