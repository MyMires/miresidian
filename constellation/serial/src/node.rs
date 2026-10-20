//! 串口通信节点及其面向测试平台的操作契约。
//!
//! 属性中的参数描述属于平台元数据，宏只原样发布；本模块仅将平台传入的实际值转换为
//! 串口操作所需的强类型。

use miresidian::{
    ByteTextFormat, DeviceInput, MiresidianError, MiresidianResult, decode_text_to_bytes,
    encode_bytes_to_text,
};

use crate::transport::{
    BufferDirection, ControlSignalState, ReceiveMode, ReceiveOptions, SerialTransport,
};

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
    #[allow(clippy::too_many_arguments)]
    #[miresidian::output("状态")]
    fn init_device(
        &mut self,
        #[miresidian::config("端口名称:COM1")] port_name: String,
        #[miresidian::config("波特率:9600")] baud_rate: u32,
        #[miresidian::config("数据位:8/7/6/5")] data_bits: u8,
        #[miresidian::config("校验位:None-无校验/Odd-奇校验/Even-偶校验")] parity: String,
        #[miresidian::config("停止位:1-1位/2-2位")] stop_bits: u8,
        #[miresidian::config("流控:None-无/Software-软件/Hardware-硬件")] flow_control: String,
        #[miresidian::config("读取超时/ms:1000")] read_timeout_ms: u64,
        #[miresidian::config("写入超时/ms:1000")] write_timeout_ms: u64,
    ) -> MiresidianResult<String> {
        let port_name = self.transport.open(
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            flow_control,
            read_timeout_ms,
            write_timeout_ms,
        )?;
        Ok(format!("已连接: {port_name}"))
    }

    /// 释放当前调用方的串口租约。
    fn close_device(&mut self) -> MiresidianResult<()> {
        self.transport.close();
        Ok(())
    }

    /// 返回系统当前可枚举到的第一个串口。
    fn get_port_no(&mut self) -> MiresidianResult<Option<String>> {
        SerialTransport::first_port()
    }

    /// 按指定数据格式将平台文本转换为字节并发送。
    #[miresidian::operation("发送数据")]
    fn send_data(
        &mut self,
        #[miresidian::input("发送数据")] data: String,
        #[miresidian::input("数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64")] format: ByteTextFormat,
    ) -> MiresidianResult<()> {
        self.transport
            .write_bytes(&decode_text_to_bytes(&data, format)?)
    }

    /// 按指定结束条件读取原始字节，并转换为平台文本。
    #[miresidian::operation("读取数据")]
    #[miresidian::output("接收数据")]
    fn read_data(
        &mut self,
        #[miresidian::input("数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64")] format: ByteTextFormat,
        #[miresidian::input("接收方式:IdleTimeout-空闲超时/Terminator-结束符/Length-固定长度")]
        receive_mode: ReceiveMode,
        #[miresidian::input("结束符:")] terminator: String,
        #[miresidian::input("接收字节数:0")] byte_count: usize,
        #[miresidian::input("接收超时/ms:0")] receive_timeout_ms: u64,
    ) -> MiresidianResult<String> {
        let options = ReceiveOptions::new(
            receive_mode,
            decode_text_to_bytes(&terminator, format)?,
            byte_count,
            receive_timeout_ms,
        )?;
        encode_bytes_to_text(&self.transport.read_bytes(options)?, format)
    }

    /// 在同一独占串口事务中完成发送、刷新、读取和结果编码。
    #[miresidian::operation("查询数据")]
    #[miresidian::output("接收数据")]
    #[allow(clippy::too_many_arguments)]
    fn query_data(
        &mut self,
        #[miresidian::input("发送数据")] data: String,
        #[miresidian::input("发送数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64")]
        send_format: ByteTextFormat,
        #[miresidian::input("接收数据格式:ASCII/GBK/UTF-8/Unicode/HEX/Base64")]
        receive_format: ByteTextFormat,
        #[miresidian::input("接收方式:IdleTimeout-空闲超时/Terminator-结束符/Length-固定长度")]
        receive_mode: ReceiveMode,
        #[miresidian::input("结束符:")] terminator: String,
        #[miresidian::input("接收字节数:0")] byte_count: usize,
        #[miresidian::input("接收超时/ms:0")] receive_timeout_ms: u64,
    ) -> MiresidianResult<String> {
        let request = decode_text_to_bytes(&data, send_format)?;
        let options = ReceiveOptions::new(
            receive_mode,
            decode_text_to_bytes(&terminator, receive_format)?,
            byte_count,
            receive_timeout_ms,
        )?;
        encode_bytes_to_text(
            &self.transport.query_bytes(&request, options)?,
            receive_format,
        )
    }

    /// 查询连接与缓冲区状态。
    #[miresidian::operation("获取串口状态")]
    #[miresidian::output("连接状态", "待读字节数", "待写字节数")]
    fn get_serial_status(&mut self) -> MiresidianResult<(String, usize, usize)> {
        let (readable, writable) = self.transport.buffered_bytes()?;
        Ok(("已连接".to_owned(), readable, writable))
    }

    /// 清除输入、输出或全部串口缓冲区。
    #[miresidian::operation("清空缓冲区")]
    fn clear_buffers(
        &mut self,
        #[miresidian::input("缓冲区方向:Input-输入/Output-输出/Both-全部")]
        direction: BufferDirection,
    ) -> MiresidianResult<()> {
        self.transport.clear_buffers(direction)
    }

    /// 按需改变 DTR、RTS 与中断信号；Keep 表示不改变对应信号。
    #[miresidian::operation("设置控制信号")]
    fn set_control_signals(
        &mut self,
        #[miresidian::input("DTR状态:Keep-保持/Enabled-开启/Disabled-关闭")]
        dtr: ControlSignalState,
        #[miresidian::input("RTS状态:Keep-保持/Enabled-开启/Disabled-关闭")]
        rts: ControlSignalState,
        #[miresidian::input("中断状态:Keep-保持/Enabled-开启/Disabled-关闭")]
        break_state: ControlSignalState,
    ) -> MiresidianResult<()> {
        self.transport.set_control_signals(dtr, rts, break_state)
    }
}

fn parse_choice<T: Copy>(value: &str, field: &str, choices: &[(&str, T)]) -> MiresidianResult<T> {
    choices
        .iter()
        .find(|(actual, _)| value.trim().eq_ignore_ascii_case(actual))
        .map(|(_, parsed)| *parsed)
        .ok_or_else(|| MiresidianError::InvalidArgument {
            field: field.to_owned(),
            message: "参数值不受支持".to_owned(),
        })
}

impl DeviceInput for ReceiveMode {
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
        parse_choice(
            value,
            field,
            &[
                ("IdleTimeout", Self::IdleTimeout),
                ("Terminator", Self::Terminator),
                ("Length", Self::Length),
            ],
        )
    }
}
impl DeviceInput for BufferDirection {
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
        parse_choice(
            value,
            field,
            &[
                ("Input", Self::Input),
                ("Output", Self::Output),
                ("Both", Self::Both),
            ],
        )
    }
}
impl DeviceInput for ControlSignalState {
    fn parse_device_text(value: &str, field: &str) -> MiresidianResult<Self> {
        parse_choice(
            value,
            field,
            &[
                ("Keep", Self::Keep),
                ("Enabled", Self::Enabled),
                ("Disabled", Self::Disabled),
            ],
        )
    }
}

#[cfg(test)]
mod ffi_tests {
    use std::ffi::CStr;

    /// 验证宏生成的最终 C ABI 入口可在未初始化时处理常驻查询。
    #[test]
    fn get_functions_无需初始化设备() {
        let output = unsafe {
            super::miresidian_invoke(c"GetFunctions".as_ptr(), c"[]".as_ptr(), c"[]".as_ptr())
        };

        let response = unsafe { CStr::from_ptr(output) }
            .to_str()
            .expect("输出必须是 UTF-8 JSON");
        assert!(response.starts_with(r#"{"status":0,"message":"","output":"#));
        assert!(response.contains("GetFunctions()(函数列表)"));
        assert!(response.ends_with(r#"]}"#));
    }

    #[test]
    fn version_queries_通过_c_abi_返回编译期版本() {
        let framework = unsafe {
            CStr::from_ptr(super::miresidian_invoke(
                c"GetFrameworkVersion".as_ptr(),
                c"[]".as_ptr(),
                c"[]".as_ptr(),
            ))
        }
        .to_str()
        .expect("框架版本响应必须是 UTF-8")
        .to_owned();
        assert_eq!(framework, r#"{"status":0,"message":"","output":["0.1.0"]}"#);

        let project = unsafe {
            CStr::from_ptr(super::miresidian_invoke(
                c"GetProjectVersion".as_ptr(),
                c"[]".as_ptr(),
                c"[]".as_ptr(),
            ))
        }
        .to_str()
        .expect("项目版本响应必须是 UTF-8");
        assert_eq!(project, r#"{"status":0,"message":"","output":["0.1.0"]}"#);
    }
}
