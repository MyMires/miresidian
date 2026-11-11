#![cfg(feature = "format")]
//! 同一节点中的旧/新格式操作共享元数据和实际调用验收，避免重复 ABI 导出。
use miresidian::{DeviceNode, Invocation, MiresidianResult, NodeRuntime};

#[miresidian::node("格式简写对照")]
struct Node;
impl Node {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("旧转换")]
    #[miresidian::output("结果", format(formats = [hex, utf8, base64, ascii, gbk, utf16le], selector = "输出格式"))]
    fn convert_long(
        &mut self,
        #[miresidian::input("数据", format(formats = [utf8, hex, base64, ascii, gbk, utf16le], selector = "输入格式"))]
        data: Vec<u8>,
    ) -> MiresidianResult<Vec<u8>> {
        Ok(data)
    }

    #[miresidian::operation("新转换")]
    #[miresidian::output(
        "结果",
        format(hex, utf8, base64, ascii, gbk, utf16le, selector = "输出格式")
    )]
    fn convert_short(
        &mut self,
        #[miresidian::input(
            "数据",
            format(utf8, hex, base64, ascii, gbk, utf16le, selector = "输入格式")
        )]
        data: Vec<u8>,
    ) -> MiresidianResult<Vec<u8>> {
        Ok(data)
    }

    #[miresidian::operation("旧固定")]
    #[miresidian::output("结果", format(formats = [hex]))]
    async fn old_fixed(
        &mut self,
        #[miresidian::input("数据", format(formats = [hex]))] data: Vec<u8>,
    ) -> MiresidianResult<Vec<u8>> {
        Ok(data)
    }

    #[miresidian::output("结果", format(hex))]
    #[miresidian::operation("新固定")]
    async fn new_fixed(
        &mut self,
        #[miresidian::input("数据", format(hex))] data: Vec<u8>,
    ) -> MiresidianResult<Vec<u8>> {
        Ok(data)
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("旧批量")]
    #[miresidian::output("结果", format(formats = [utf8, hex, base64], selector = "输出格式"))]
    async fn old_batch(
        &mut self,
        #[miresidian::input("数据", batch, format(formats = [utf8, hex, base64], selector = "输入格式"))]
        data: String,
    ) -> MiresidianResult<String> {
        Ok(data)
    }

    #[cfg(feature = "batch")]
    #[miresidian::operation("新批量")]
    #[miresidian::output("结果", format(utf8, hex, base64, selector = "输出格式"))]
    async fn new_batch(
        &mut self,
        #[miresidian::input("数据", format(utf8, hex, base64, selector = "输入格式"), batch)] data: String,
    ) -> MiresidianResult<String> {
        Ok(data)
    }
}
fn request(operation: &str, input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        vec![],
        input.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )
}
fn runtime() -> NodeRuntime<Node> {
    let runtime = NodeRuntime::new();
    runtime.invoke(&request("InitDevice", &[])).unwrap();
    runtime
}
fn same_metadata(old: &str, new: &str) {
    let descriptor = Node::descriptor();
    let old = descriptor
        .operations
        .iter()
        .find(|op| op.name == old)
        .unwrap();
    let new = descriptor
        .operations
        .iter()
        .find(|op| op.name == new)
        .unwrap();
    assert_eq!(old.inputs, new.inputs);
    assert_eq!(old.outputs, new.outputs);
    assert_eq!(old.channel_inputs, new.channel_inputs);
}
#[test]
fn formats_selectors_defaults_and_errors_are_identical() {
    same_metadata("旧转换", "新转换");
    same_metadata("旧固定", "新固定");
    let runtime = runtime();
    for (format, data, expected) in [
        ("ASCII", "A", "41"),
        ("GBK", "A", "41"),
        ("UTF8", "A", "41"),
        ("Unicode", "A", "41 00"),
        ("HEX", "41", "41"),
        ("Base64", "QQ==", "41"),
    ] {
        for operation in ["旧转换", "新转换"] {
            assert_eq!(
                runtime
                    .invoke(&request(operation, &[data, format, "HEX"]))
                    .unwrap()
                    .as_slice(),
                [expected]
            );
        }
    }
    for operation in ["旧转换", "新转换"] {
        assert_eq!(
            runtime
                .invoke(&request(operation, &["A", "", ""]))
                .unwrap()
                .as_slice(),
            ["41"]
        );
        assert!(
            runtime
                .invoke(&request(operation, &["ZZ", "HEX", "HEX"]))
                .is_err()
        );
        assert!(
            runtime
                .invoke(&request(operation, &["A", "unknown", "HEX"]))
                .is_err()
        );
    }
    for operation in ["旧固定", "新固定"] {
        assert_eq!(
            runtime
                .invoke(&request(operation, &["41 42"]))
                .unwrap()
                .as_slice(),
            ["41 42"]
        );
    }
    runtime.shutdown().unwrap();
}
#[cfg(feature = "batch")]
#[test]
fn async_batch_record_and_output_format_columns_are_identical() {
    same_metadata("旧批量", "新批量");
    let runtime = runtime();
    for operation in ["旧批量", "新批量"] {
        assert_eq!(
            runtime
                .invoke(&request(
                    operation,
                    &["A|42|Qw==", "UTF8|HEX|Base64", "UTF8|HEX|Base64"]
                ))
                .unwrap()
                .as_slice(),
            ["A|42|base64:Qw=="]
        );
        assert!(
            runtime
                .invoke(&request(operation, &["A|42|Qw==", "UTF8|HEX", "UTF8"]))
                .is_err()
        );
    }
    runtime.shutdown().unwrap();
}
#[test]
fn short_format_reaches_the_generated_complete_json_entry() {
    use std::ffi::{CStr, CString};
    let call = |request: &str| {
        let request = CString::new(request).unwrap();
        // SAFETY: 本次调用持有 NUL 终止的请求，下一次 ABI 调用前复制 DLL 响应。
        let response = unsafe { CStr::from_ptr(miresidian_invoke_request(request.as_ptr())) };
        serde_json::from_slice::<serde_json::Value>(response.to_bytes()).unwrap()
    };
    assert_eq!(call(r#"{"function":"InitDevice"}"#)["code"], "OK");
    let response = call(r#"{"function":"新固定","input":["41 42"]}"#);
    assert_eq!(response["output"], serde_json::json!(["41 42"]));
    assert_eq!(call(r#"{"function":"CloseDevice"}"#)["code"], "OK");
}
