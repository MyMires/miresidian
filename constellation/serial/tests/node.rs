//! 串口通信节点面向测试平台的节点契约测试。

use miresidian::{DeviceNode, Invocation, NodeRuntime};
use miresidian_serial::SerialNode;

#[test]
fn 方法标注会生成中文平台契约() {
    let signatures = SerialNode::descriptor().function_signatures();

    assert!(signatures.contains(&"写入文本(发送文本)(写入字节数)".to_owned()));
    assert!(signatures.contains(&"读取文本(最大接收字节数:1024)(接收文本)".to_owned()));
    assert!(signatures.contains(&"查询文本(发送文本,最大接收字节数:1024)(接收文本)".to_owned()));
}

#[test]
fn 固定操作由宏生成且无需初始化() {
    let mut runtime = NodeRuntime::<SerialNode>::new();
    let output = runtime
        .invoke(&Invocation::new("GetInitConfig", Vec::new(), Vec::new()))
        .expect("GetInitConfig 应由宏生成");

    assert_eq!(output.as_slice()[0], "端口名称:COM1");
    assert_eq!(output.as_slice()[1], "波特率:9600");
}
