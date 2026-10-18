//! 串口通信节点及其面向测试平台的操作契约。
//!
//! 节点状态和平台操作共同构成一个完整的设备节点，属性宏据此生成生命周期调度、操作描述
//! 和 C ABI 入口。

use miresidian::{ByteTextFormat, DriverResult, decode_text_to_bytes, encode_bytes_to_text};

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
        #[miresidian::config("校验位:None-无校验/Odd-奇校验/Even-偶校验")] parity: String,
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

    /// 按指定数据格式将平台文本转换为字节并发送。
    #[miresidian::operation("发送数据")]
    fn send_data(
        &mut self,
        #[miresidian::input("发送数据")] data: String,
        #[miresidian::input("数据格式:UTF-8/Unicode/HEX/Base64")] format: ByteTextFormat,
    ) -> DriverResult<()> {
        let bytes = decode_text_to_bytes(&data, format)?;
        self.transport.write_bytes(&bytes)
    }

    /// 读取字节直到本次读取超时，再编码为平台文本。
    #[miresidian::operation("读取数据")]
    #[miresidian::output("接收数据")]
    fn read_data(
        &mut self,
        #[miresidian::input("数据格式:UTF-8/Unicode/HEX/Base64")] format: ByteTextFormat,
    ) -> DriverResult<String> {
        encode_bytes_to_text(&self.transport.read_bytes()?, format)
    }

    /// 在单一独占事务内完成发送、刷新、读取和结果编码。
    #[miresidian::operation("查询数据")]
    #[miresidian::output("接收数据")]
    fn query_data(
        &mut self,
        #[miresidian::input("发送数据")] data: String,
        #[miresidian::input("发送数据格式:UTF-8/Unicode/HEX/Base64")] send_format: ByteTextFormat,
        #[miresidian::input("接收数据格式:UTF-8/Unicode/HEX/Base64")]
        receive_format: ByteTextFormat,
    ) -> DriverResult<String> {
        let request = decode_text_to_bytes(&data, send_format)?;
        encode_bytes_to_text(&self.transport.query_bytes(&request)?, receive_format)
    }
}
