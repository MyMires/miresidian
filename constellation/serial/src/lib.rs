//! 由属性标注声明的串口通信驱动。
//!
//! 本 crate 不手写 `GetFunctions`、参数元数据、字符串分发或 C ABI 入口；这些均由
//! `#[miresidian::node_impl]` 根据下列方法标注生成。

use miresidian::{DriverResult, SerialTransport};

/// 面向测试平台的通用串口通信驱动。
#[miresidian::node("串口通信")]
#[derive(Default)]
pub struct SerialDriver {
    transport: SerialTransport,
}

#[miresidian::node_impl]
impl SerialDriver {
    /// 使用平台提供的串口配置建立或取得共享连接。
    #[miresidian::output("状态")]
    fn init_device(
        &mut self,
        #[miresidian::config("端口名称:COM1")] port_name: String,
        #[miresidian::config("波特率:9600")] baud_rate: u32,
        #[miresidian::config("数据位:8")] data_bits: u8,
        #[miresidian::config("校验位:None")] parity: String,
        #[miresidian::config("停止位:1")] stop_bits: u8,
        #[miresidian::config("流控:None")] flow_control: String,
        #[miresidian::config("超时毫秒:1000")] timeout_ms: u64,
    ) -> DriverResult<String> {
        let port_name = self.transport.open(
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            flow_control,
            timeout_ms,
        )?;
        Ok(format!("已连接: {port_name}"))
    }

    /// 释放当前调用方的串口租约。
    fn close_device(&mut self) -> DriverResult<()> {
        self.transport.close();
        Ok(())
    }

    /// 返回系统当前可枚举到的第一个串口。
    fn get_port_no(&mut self) -> DriverResult<Option<String>> {
        SerialTransport::first_port()
    }

    /// 将 UTF-8 文本写入串口。
    #[miresidian::operation("写入文本")]
    #[miresidian::output("写入字节数")]
    fn write_text(
        &mut self, #[miresidian::input("发送文本")] text: String
    ) -> DriverResult<usize> {
        self.transport.write_text(text)
    }

    /// 从串口读取 UTF-8 文本。
    #[miresidian::operation("读取文本")]
    #[miresidian::output("接收文本")]
    fn read_text(
        &mut self,
        #[miresidian::input("最大接收字节数:1024")] maximum_bytes: usize,
    ) -> DriverResult<String> {
        self.transport.read_text(maximum_bytes)
    }

    /// 在单一独占事务内完成写入与读取。
    #[miresidian::operation("查询文本")]
    #[miresidian::output("接收文本")]
    fn query_text(
        &mut self,
        #[miresidian::input("发送文本")] text: String,
        #[miresidian::input("最大接收字节数:1024")] maximum_bytes: usize,
    ) -> DriverResult<String> {
        self.transport.query_text(text, maximum_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use miresidian::{DeviceNode, Invocation, NodeRuntime};

    #[test]
    fn 方法标注会生成中文平台契约() {
        let signatures = SerialDriver::descriptor().function_signatures();

        assert!(signatures.contains(&"写入文本(发送文本)(写入字节数)".to_owned()));
        assert!(signatures.contains(&"读取文本(最大接收字节数:1024)(接收文本)".to_owned()));
        assert!(
            signatures.contains(&"查询文本(发送文本,最大接收字节数:1024)(接收文本)".to_owned())
        );
    }

    #[test]
    fn 固定操作由宏生成且无需初始化() {
        let mut runtime = NodeRuntime::<SerialDriver>::new();
        let output = runtime
            .invoke(&Invocation::new("GetInitConfig", Vec::new(), Vec::new()))
            .expect("GetInitConfig 应由宏生成");

        assert_eq!(output.as_slice()[0], "端口名称:COM1");
        assert_eq!(output.as_slice()[1], "波特率:9600");
    }
}
