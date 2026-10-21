//! 串口通信节点面向测试平台的节点契约测试。

use miresidian::{DeviceNode, Invocation, MIRESIDIAN_VERSION, NodeRuntime};
use miresidian_serial::SerialNode;

#[test]
fn 方法标注会生成中文平台契约() {
    let signatures = SerialNode::descriptor().function_signatures();

    assert!(
        signatures.contains(
            &"发送数据(发送数据,数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64)()".to_owned()
        )
    );
    assert!(signatures.contains(
        &"读取数据(数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64,接收方式:IdleTimeout-空闲超时/Terminator-结束符/Length-固定长度,结束符:,接收字节数:0,接收超时/ms:0)(接收数据)".to_owned()
    ));
    assert!(signatures.contains(
        &"查询数据(发送数据,发送数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64,接收数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64,接收方式:IdleTimeout-空闲超时/Terminator-结束符/Length-固定长度,结束符:,接收字节数:0,接收超时/ms:0)(接收数据)".to_owned()
    ));
}

#[test]
fn 固定操作由宏生成且无需初始化() {
    let mut runtime = NodeRuntime::<SerialNode>::new();
    let output = runtime
        .invoke(&Invocation::new("GetInitConfig", Vec::new(), Vec::new()))
        .expect("GetInitConfig 应由宏生成");

    assert_eq!(output.as_slice()[0], "端口名称:COM1");
    assert_eq!(output.as_slice()[1], "波特率:9600");
    assert_eq!(
        output.as_slice()[3],
        "校验位:None-无校验/Odd-奇校验/Even-偶校验"
    );
}

#[test]
fn 版本查询由宏生成且无需初始化() {
    let mut runtime = NodeRuntime::<SerialNode>::new();

    let framework = runtime
        .invoke(&Invocation::new(
            "GetFrameworkVersion",
            Vec::new(),
            Vec::new(),
        ))
        .expect("GetFrameworkVersion 应由框架处理");
    assert_eq!(framework.as_slice(), [MIRESIDIAN_VERSION]);

    let project = runtime
        .invoke(&Invocation::new(
            "GetProjectVersion",
            Vec::new(),
            Vec::new(),
        ))
        .expect("GetProjectVersion 应由宏生成");
    assert_eq!(project.as_slice(), ["0.2.0"]);
    assert!(!runtime.is_initialized());
}
