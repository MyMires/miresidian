//! 属性宏生成代码复用根框架数据编解码 API 的集成测试。

use miresidian::{ByteTextFormat, DriverResult, Invocation, NodeRuntime, encode_bytes_to_text};

#[miresidian::node("编解码宏测试节点")]
#[derive(Default)]
struct CodecMacroNode;

#[miresidian::node_impl]
impl CodecMacroNode {
    #[miresidian::output("初始化格式")]
    fn init_device(
        &mut self,
        #[miresidian::config("初始化格式:GBK")] format: ByteTextFormat,
    ) -> DriverResult<String> {
        Ok(format.to_string())
    }

    fn close_device(&mut self) -> DriverResult<()> {
        Ok(())
    }

    #[miresidian::operation("编解码回显")]
    #[miresidian::output("实际格式", "规范数据")]
    fn codec_echo(
        &mut self,
        #[miresidian::input("数据")] data: Vec<u8>,
        #[miresidian::input("数据格式")] format: ByteTextFormat,
    ) -> DriverResult<(String, String)> {
        Ok((format.to_string(), encode_bytes_to_text(&data, format)?))
    }
}

#[test]
fn 属性宏生成的调度复用根框架编解码规则() {
    let mut runtime = NodeRuntime::<CodecMacroNode>::new();
    let initialized = runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec!["初始化格式:ASCII".to_owned()],
            Vec::new(),
        ))
        .expect("宏生成的 Config 调度应解析 ByteTextFormat");
    assert_eq!(initialized.as_slice(), ["ASCII"]);

    let output = runtime
        .invoke(&Invocation::new(
            "编解码回显",
            Vec::new(),
            vec!["base64:AP8K".to_owned(), "HEX".to_owned()],
        ))
        .expect("宏生成的 Input 调度应解析 Vec<u8> 与 ByteTextFormat");
    assert_eq!(output.as_slice(), ["HEX", "00 FF 0A"]);
}
