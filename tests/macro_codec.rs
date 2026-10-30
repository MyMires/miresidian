//! 属性宏生成代码复用根框架数据编解码 API 的集成测试。

use miresidian::{ByteTextFormat, Invocation, MiresidianResult, NodeRuntime, encode_bytes_to_text};

#[cfg_attr(feature = "instances", miresidian::node("编解码宏测试节点", instances(mode = multiple, binding = config, execution = sequential)))]
#[cfg_attr(not(feature = "instances"), miresidian::node("编解码宏测试节点"))]
struct CodecMacroNode {
    connection: String,
}

impl CodecMacroNode {
    #[miresidian::operation]
    #[miresidian::output("初始化格式")]
    fn init_device(
        #[miresidian::config("传输方式:Serial-串口/Tcp-网络")] transport: String,
        #[miresidian::config("地址:127.0.0.1")] address: String,
        #[miresidian::config("初始化格式:GBK")] format: ByteTextFormat,
    ) -> MiresidianResult<(Self, String)> {
        Ok((
            Self {
                connection: format!("{transport}:{address}"),
            },
            format.to_string(),
        ))
    }

    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }

    #[miresidian::operation("读取连接")]
    fn read_connection(&mut self) -> MiresidianResult<String> {
        Ok(self.connection.clone())
    }

    #[miresidian::operation("编解码回显")]
    #[miresidian::output("实际格式", "规范数据")]
    fn codec_echo(
        &mut self,
        #[miresidian::input("数据")] data: Vec<u8>,
        #[miresidian::input("数据格式:HEX-十六进制/ASCII-文本")] format: ByteTextFormat,
    ) -> MiresidianResult<(String, String)> {
        Ok((format.to_string(), encode_bytes_to_text(&data, format)?))
    }
}

#[test]
fn 属性宏生成的调度复用根框架编解码规则() {
    let runtime = NodeRuntime::<CodecMacroNode>::new();
    let initialized = runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec![
                "Serial".to_owned(),
                "127.0.0.1".to_owned(),
                "初始化格式:ASCII".to_owned(),
            ],
            Vec::new(),
        ))
        .expect("宏生成的 Config 调度应解析 ByteTextFormat");
    assert_eq!(initialized.as_slice(), ["ASCII"]);

    let output = runtime
        .invoke(&Invocation::new(
            "编解码回显",
            vec![
                "Serial".to_owned(),
                "127.0.0.1".to_owned(),
                "初始化格式:ASCII".to_owned(),
            ],
            vec!["base64:AP8K".to_owned(), "十六进制".to_owned()],
        ))
        .expect("宏生成的 Input 调度应解析 Vec<u8> 与 ByteTextFormat");
    assert_eq!(output.as_slice(), ["HEX", "00 FF 0A"]);
}

#[test]
fn 等价业务配置定位并复用同一通道() {
    let runtime = NodeRuntime::<CodecMacroNode>::new();
    let initialized = runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec!["串口".to_owned()],
            Vec::new(),
        ))
        .expect("显示值与缺省配置应可初始化");
    assert_eq!(initialized.as_slice(), ["GBK"]);

    let reused = runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec![
                "传输方式:Serial".to_owned(),
                "地址:127.0.0.1".to_owned(),
                "初始化格式:GBK".to_owned(),
            ],
            Vec::new(),
        ))
        .expect("参数名前缀、实际值和显式默认值应命中已有通道");
    assert_eq!(reused.as_slice(), [miresidian::CHANNEL_REUSED_MESSAGE]);

    let connection = runtime
        .invoke(&Invocation::new(
            "读取连接",
            vec!["传输方式:串口".to_owned()],
            Vec::new(),
        ))
        .expect("等价 Config 应定位到已初始化通道");
    assert_eq!(connection.as_slice(), ["Serial:127.0.0.1"]);
}

#[test]
#[cfg(feature = "instances")]
fn 共享调用同样使用枚举显示值和默认值定位对象() {
    let runtime = NodeRuntime::<CodecMacroNode>::new();
    runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec!["Serial".to_owned()],
            vec![],
        ))
        .unwrap();
    let response = runtime
        .invoke_command(miresidian::RuntimeInvocation::MultiChannel(
            miresidian::MultiChannelInvocation::new(
                "读取连接",
                vec!["传输方式:串口".to_owned()],
                vec![Some(vec![]), Some(vec![])],
            ),
        ))
        .unwrap();
    let miresidian::RuntimeResponse::MultiChannel(response) = response else {
        panic!("共享调用应返回结构化结果");
    };
    assert_eq!(response.slots.len(), 2);
    for output in response.slots {
        assert_eq!(output.unwrap().as_slice(), ["Serial:127.0.0.1"]);
    }
}
