//! 串口传输实现。
//!
//! 本模块负责连接、字节收发和跨 DLL 租约；平台方法及其描述由 `node` 声明。

use miresidian::{MiresidianError, MiresidianResult};
use serialport::{DataBits, FlowControl, Parity, StopBits};

use crate::lease::SerialLease;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReceiveMode {
    IdleTimeout,
    Terminator,
    Length,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BufferDirection {
    Input,
    Output,
    Both,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ControlSignalState {
    Keep,
    Enabled,
    Disabled,
}

/// 一次读取的结束条件。`timeout_ms == 0` 表示使用初始化读取超时。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReceiveOptions {
    pub(super) mode: ReceiveMode,
    pub(super) terminator: Vec<u8>,
    pub(super) byte_count: usize,
    pub(super) timeout_ms: u64,
}
impl ReceiveOptions {
    pub(super) fn new(
        mode: ReceiveMode,
        terminator: Vec<u8>,
        byte_count: usize,
        timeout_ms: u64,
    ) -> MiresidianResult<Self> {
        if matches!(mode, ReceiveMode::Terminator) && terminator.is_empty() {
            return Err(invalid_value("结束符", "结束符模式必须提供结束符"));
        }
        if matches!(mode, ReceiveMode::Length) && byte_count == 0 {
            return Err(invalid_value("接收字节数", "固定长度模式必须大于 0"));
        }
        Ok(Self {
            mode,
            terminator,
            byte_count,
            timeout_ms,
        })
    }
}

#[derive(Default)]
pub struct SerialTransport {
    lease: Option<SerialLease>,
}
impl SerialTransport {
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        &mut self,
        port_name: String,
        baud_rate: u32,
        data_bits: u8,
        parity: String,
        stop_bits: u8,
        flow_control: String,
        read_timeout_ms: u64,
        write_timeout_ms: u64,
    ) -> MiresidianResult<String> {
        self.close();
        let settings = SerialSettings::new(
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            flow_control,
            read_timeout_ms,
            write_timeout_ms,
        )?;
        let connected_port = settings.port_name.clone();
        self.lease = Some(SerialLease::acquire(&settings)?);
        Ok(connected_port)
    }
    pub fn close(&mut self) {
        self.lease.take();
    }
    pub fn write_bytes(&mut self, bytes: &[u8]) -> MiresidianResult<()> {
        self.lease_mut()?.write(bytes)
    }
    pub fn read_bytes(&mut self, options: ReceiveOptions) -> MiresidianResult<Vec<u8>> {
        self.lease_mut()?.read(options)
    }
    pub fn query_bytes(
        &mut self,
        bytes: &[u8],
        options: ReceiveOptions,
    ) -> MiresidianResult<Vec<u8>> {
        self.lease_mut()?.query(bytes, options)
    }
    pub fn clear_buffers(&mut self, direction: BufferDirection) -> MiresidianResult<()> {
        self.lease_mut()?.clear(direction)
    }
    pub fn set_control_signals(
        &mut self,
        dtr: ControlSignalState,
        rts: ControlSignalState,
        break_state: ControlSignalState,
    ) -> MiresidianResult<()> {
        self.lease_mut()?.set_control(dtr, rts, break_state)
    }
    pub fn buffered_bytes(&mut self) -> MiresidianResult<(usize, usize)> {
        self.lease_mut()?.buffered_bytes()
    }
    pub fn first_port() -> MiresidianResult<Option<String>> {
        Ok(serialport::available_ports()
            .map_err(|error| serial_failure("枚举", error))?
            .into_iter()
            .next()
            .map(|port| port.port_name))
    }
    fn lease_mut(&mut self) -> MiresidianResult<&mut SerialLease> {
        self.lease.as_mut().ok_or(MiresidianError::NotInitialized)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SerialSettings {
    pub(super) port_name: String,
    pub(super) baud_rate: u32,
    pub(super) data_bits: DataBits,
    pub(super) parity: Parity,
    pub(super) stop_bits: StopBits,
    pub(super) flow_control: FlowControl,
    pub(super) read_timeout_ms: u64,
    pub(super) write_timeout_ms: u64,
}
impl SerialSettings {
    #[allow(clippy::too_many_arguments)]
    fn new(
        port_name: String,
        baud_rate: u32,
        data_bits: u8,
        parity: String,
        stop_bits: u8,
        flow_control: String,
        read_timeout_ms: u64,
        write_timeout_ms: u64,
    ) -> MiresidianResult<Self> {
        let port_name = port_name.trim().to_owned();
        if port_name.is_empty() {
            return Err(invalid_value("端口名称", "不能为空"));
        }
        if read_timeout_ms == 0 {
            return Err(invalid_value("读取超时/ms", "必须大于 0"));
        }
        if write_timeout_ms == 0 {
            return Err(invalid_value("写入超时/ms", "必须大于 0"));
        }
        Ok(Self {
            port_name,
            baud_rate,
            data_bits: parse_data_bits(data_bits)?,
            parity: parse_parity(&parity)?,
            stop_bits: parse_stop_bits(stop_bits)?,
            flow_control: parse_flow_control(&flow_control)?,
            read_timeout_ms,
            write_timeout_ms,
        })
    }
}
fn parse_data_bits(value: u8) -> MiresidianResult<DataBits> {
    match value {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        _ => Err(invalid_value("数据位", "仅支持 5、6、7 或 8")),
    }
}
fn parse_parity(value: &str) -> MiresidianResult<Parity> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(Parity::None),
        "odd" => Ok(Parity::Odd),
        "even" => Ok(Parity::Even),
        _ => Err(invalid_value("校验位", "仅支持 None、Odd 或 Even")),
    }
}
fn parse_stop_bits(value: u8) -> MiresidianResult<StopBits> {
    match value {
        1 => Ok(StopBits::One),
        2 => Ok(StopBits::Two),
        _ => Err(invalid_value("停止位", "仅支持 1 或 2")),
    }
}
fn parse_flow_control(value: &str) -> MiresidianResult<FlowControl> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(FlowControl::None),
        "hardware" => Ok(FlowControl::Hardware),
        "software" => Ok(FlowControl::Software),
        _ => Err(invalid_value("流控", "仅支持 None、Hardware 或 Software")),
    }
}
fn invalid_value(field: &str, message: &str) -> MiresidianError {
    MiresidianError::InvalidArgument {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}
pub(super) fn serial_failure(action: &str, error: serialport::Error) -> MiresidianError {
    MiresidianError::Failure(format!("串口{action}失败: {error}"))
}
pub(super) fn io_failure(action: &str, error: std::io::Error) -> MiresidianError {
    MiresidianError::Failure(format!("串口{action}失败: {error}"))
}
