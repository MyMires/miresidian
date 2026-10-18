//! 串口传输实现。
//!
//! 本模块只负责串口连接、字节收发及跨 DLL 租约；它不声明任何测试平台功能。
//! 平台方法、参数名称和输出契约全部由具体驱动的方法属性宏声明。

use serialport::{DataBits, FlowControl, Parity, StopBits};

use crate::lease::SerialLease;

use miresidian::{DriverError, DriverResult};

/// 可被具体设备驱动组合使用的串口会话。
#[derive(Default)]
pub struct SerialTransport {
    lease: Option<SerialLease>,
}

impl SerialTransport {
    /// 按给定通信参数取得串口租约，并返回已连接的端口名称。
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        &mut self,
        port_name: String,
        baud_rate: u32,
        data_bits: u8,
        parity: String,
        stop_bits: u8,
        flow_control: String,
        timeout_ms: u64,
    ) -> DriverResult<String> {
        self.close();
        let settings = SerialSettings::new(
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            flow_control,
            timeout_ms,
        )?;
        let connected_port = settings.port_name.clone();
        self.lease = Some(SerialLease::acquire(&settings)?);
        Ok(connected_port)
    }

    /// 释放当前调用方持有的串口租约。
    pub fn close(&mut self) {
        self.lease.take();
    }

    /// 写入并刷新原始字节。
    pub fn write_bytes(&mut self, bytes: &[u8]) -> DriverResult<()> {
        self.lease_mut()?.write(bytes)
    }

    /// 读取原始字节，直到本次读取超时。
    pub fn read_bytes(&mut self) -> DriverResult<Vec<u8>> {
        self.lease_mut()?.read()
    }

    /// 在同一串口事务中写入、刷新并读取原始字节响应。
    pub fn query_bytes(&mut self, bytes: &[u8]) -> DriverResult<Vec<u8>> {
        self.lease_mut()?.query(bytes)
    }

    /// 返回系统枚举的第一个串口；具体驱动可按设备协议实现更严格的探测。
    pub fn first_port() -> DriverResult<Option<String>> {
        let ports = serialport::available_ports().map_err(|error| serial_failure("枚举", error))?;
        Ok(ports.into_iter().next().map(|port| port.port_name))
    }

    fn lease_mut(&mut self) -> DriverResult<&mut SerialLease> {
        self.lease.as_mut().ok_or(DriverError::NotInitialized)
    }
}

/// 已验证的串口物理连接参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SerialSettings {
    pub(super) port_name: String,
    pub(super) baud_rate: u32,
    pub(super) data_bits: DataBits,
    pub(super) parity: Parity,
    pub(super) stop_bits: StopBits,
    pub(super) flow_control: FlowControl,
    pub(super) timeout_ms: u64,
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
        timeout_ms: u64,
    ) -> DriverResult<Self> {
        let port_name = port_name.trim().to_owned();
        if port_name.is_empty() {
            return Err(invalid_value("端口名称", "不能为空"));
        }
        if timeout_ms == 0 {
            return Err(invalid_value("超时毫秒", "必须大于 0"));
        }
        Ok(Self {
            port_name,
            baud_rate,
            data_bits: parse_data_bits(data_bits)?,
            parity: parse_parity(&parity)?,
            stop_bits: parse_stop_bits(stop_bits)?,
            flow_control: parse_flow_control(&flow_control)?,
            timeout_ms,
        })
    }
}

fn parse_data_bits(value: u8) -> DriverResult<DataBits> {
    match value {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        _ => Err(invalid_value("数据位", "仅支持 5、6、7 或 8")),
    }
}

fn parse_parity(value: &str) -> DriverResult<Parity> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" | "无" | "无校验" => Ok(Parity::None),
        "odd" | "奇" | "奇校验" => Ok(Parity::Odd),
        "even" | "偶" | "偶校验" => Ok(Parity::Even),
        _ => Err(invalid_value("校验位", "仅支持 None、Odd 或 Even")),
    }
}

fn parse_stop_bits(value: u8) -> DriverResult<StopBits> {
    match value {
        1 => Ok(StopBits::One),
        2 => Ok(StopBits::Two),
        _ => Err(invalid_value("停止位", "仅支持 1 或 2")),
    }
}

fn parse_flow_control(value: &str) -> DriverResult<FlowControl> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" | "无" | "无流控" => Ok(FlowControl::None),
        "hardware" | "硬件" | "硬件流控" => Ok(FlowControl::Hardware),
        "software" | "软件" | "软件流控" => Ok(FlowControl::Software),
        _ => Err(invalid_value("流控", "仅支持 None、Hardware 或 Software")),
    }
}

fn invalid_value(field: &str, message: &str) -> DriverError {
    DriverError::InvalidArgument {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

pub(super) fn serial_failure(action: &str, error: serialport::Error) -> DriverError {
    DriverError::Failure(format!("串口{action}失败: {error}"))
}

pub(super) fn io_failure(action: &str, error: std::io::Error) -> DriverError {
    DriverError::Failure(format!("串口{action}失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 拒绝不支持的串口参数() {
        assert!(
            SerialSettings::new(
                "COM1".to_owned(),
                115_200,
                9,
                "None".to_owned(),
                1,
                "None".to_owned(),
                1_000,
            )
            .is_err()
        );
    }

    #[test]
    fn 平台传入的校验位实际值会被正确解析() {
        let settings = SerialSettings::new(
            "COM1".to_owned(),
            9_600,
            8,
            "Odd".to_owned(),
            1,
            "None".to_owned(),
            1_000,
        )
        .expect("Odd 是平台调用阶段传入的有效校验位值");

        assert_eq!(settings.parity, Parity::Odd);
    }
}
