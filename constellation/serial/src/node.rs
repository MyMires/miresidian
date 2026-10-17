//! 串口通信节点及其面向测试平台的操作契约。
//!
//! 节点状态和平台操作共同构成一个完整的设备节点，属性宏据此生成生命周期调度、操作描述
//! 和 C ABI 入口。

use miresidian::DriverResult;

use crate::transport::SerialTransport;

/// 面向测试平台的通用串口通信节点。
#[miresidian::node("串口通信")]
#[derive(Default)]
pub struct SerialNode {
    /// 当前 DLL 调用方持有的串口会话；会话内部管理跨 DLL 租约。
    transport: SerialTransport,
}

#[miresidian::node_impl]
impl SerialNode {
    /// 使用平台提供的串口配置建立或取得共享连接。
    /// 初始化配置需逐项公开给测试平台，方法签名与平台契约保持一一对应。
    #[allow(clippy::too_many_arguments)]
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
