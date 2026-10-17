//! 基于 `serialport` crate 的串口通信节点。
//!
//! 本模块只处理串口的打开、参数设置和 UTF-8 文本收发，不包含任何具体仪器协议。
//! 具体设备驱动可在此节点之上定义自己的帧格式、校验和与命令语义。

use std::time::Duration;

use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

use crate::{
    DeviceNode, DriverError, DriverResult, InvocationOutput, NodeDescriptor, OperationDescriptor,
    ParameterDescriptor,
};

const DEFAULT_PORT_NAME: &str = "COM1";
const DEFAULT_BAUD_RATE_TEXT: &str = "115200";
const DEFAULT_TIMEOUT_MS_TEXT: &str = "1000";
const DEFAULT_MAXIMUM_BYTES_TEXT: &str = "1024";

const INIT_CONFIG: [ParameterDescriptor; 7] = [
    ParameterDescriptor {
        name: "端口名称",
        default_value: Some(DEFAULT_PORT_NAME),
    },
    ParameterDescriptor {
        name: "波特率",
        default_value: Some(DEFAULT_BAUD_RATE_TEXT),
    },
    ParameterDescriptor {
        name: "数据位",
        default_value: Some("8"),
    },
    ParameterDescriptor {
        name: "校验位",
        default_value: Some("None"),
    },
    ParameterDescriptor {
        name: "停止位",
        default_value: Some("1"),
    },
    ParameterDescriptor {
        name: "流控",
        default_value: Some("None"),
    },
    ParameterDescriptor {
        name: "超时毫秒",
        default_value: Some(DEFAULT_TIMEOUT_MS_TEXT),
    },
];

const INIT_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "状态",
    default_value: None,
}];

const WRITE_INPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "发送文本",
    default_value: None,
}];
const WRITE_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "写入字节数",
    default_value: None,
}];
const READ_INPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "最大接收字节数",
    default_value: Some(DEFAULT_MAXIMUM_BYTES_TEXT),
}];
const READ_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "接收文本",
    default_value: None,
}];
const QUERY_INPUTS: [ParameterDescriptor; 2] = [
    ParameterDescriptor {
        name: "发送文本",
        default_value: None,
    },
    ParameterDescriptor {
        name: "最大接收字节数",
        default_value: Some(DEFAULT_MAXIMUM_BYTES_TEXT),
    },
];
const QUERY_OUTPUTS: [ParameterDescriptor; 1] = [ParameterDescriptor {
    name: "接收文本",
    default_value: None,
}];

const OPERATIONS: [OperationDescriptor; 3] = [
    OperationDescriptor {
        name: "WriteUtf8",
        inputs: &WRITE_INPUTS,
        outputs: &WRITE_OUTPUTS,
    },
    OperationDescriptor {
        name: "ReadUtf8",
        inputs: &READ_INPUTS,
        outputs: &READ_OUTPUTS,
    },
    OperationDescriptor {
        name: "QueryUtf8",
        inputs: &QUERY_INPUTS,
        outputs: &QUERY_OUTPUTS,
    },
];

const DESCRIPTOR: NodeDescriptor = NodeDescriptor {
    name: "串口通信",
    init_config: &INIT_CONFIG,
    init_outputs: &INIT_OUTPUTS,
    operations: &OPERATIONS,
};

/// 可供测试平台直接调用的串口通信节点。
///
/// `WriteUtf8`、`ReadUtf8` 与 `QueryUtf8` 只传递严格有效的 UTF-8 文本。
/// 二进制协议应在后续字节数组契约完成后使用专门的字节操作，避免静默损坏数据。
#[derive(Default)]
pub struct SerialNode {
    port: Option<Box<dyn SerialPort>>,
}

impl DeviceNode for SerialNode {
    fn descriptor() -> &'static NodeDescriptor {
        &DESCRIPTOR
    }

    fn init_device(&mut self, config: &[String]) -> DriverResult<InvocationOutput> {
        let settings = SerialSettings::from_config(config)?;
        let port_name = settings.port_name.clone();
        let port = serialport::new(&settings.port_name, settings.baud_rate)
            .data_bits(settings.data_bits)
            .parity(settings.parity)
            .stop_bits(settings.stop_bits)
            .flow_control(settings.flow_control)
            .timeout(Duration::from_millis(settings.timeout_ms))
            .open()
            .map_err(|error| serial_failure("打开", error))?;

        self.port = Some(port);
        Ok(InvocationOutput::new(vec![format!("已连接: {port_name}")]))
    }

    fn close_device(&mut self) -> DriverResult<()> {
        // 丢弃端口对象即可关闭操作系统句柄；不存在可恢复的关闭失败路径。
        self.port.take();
        Ok(())
    }

    fn invoke(
        &mut self,
        operation: &str,
        _config: &[String],
        input: &[String],
    ) -> DriverResult<InvocationOutput> {
        match operation {
            "WriteUtf8" => self.write_utf8(required_input(input, 0, "发送文本")?),
            "ReadUtf8" => self.read_utf8(maximum_bytes(input, 0)?),
            "QueryUtf8" => {
                let text = required_input(input, 0, "发送文本")?;
                let maximum_bytes = maximum_bytes(input, 1)?;
                self.write_all(text.as_bytes())?;
                self.read_utf8(maximum_bytes)
            }
            other => Err(DriverError::InvalidOperation(other.to_owned())),
        }
    }

    fn get_port_no(&mut self) -> DriverResult<Option<String>> {
        let ports = serialport::available_ports().map_err(|error| serial_failure("枚举", error))?;
        Ok(ports.into_iter().next().map(|port| port.port_name))
    }
}

impl SerialNode {
    fn write_utf8(&mut self, text: &str) -> DriverResult<InvocationOutput> {
        self.write_all(text.as_bytes())?;
        Ok(InvocationOutput::new(vec![text.len().to_string()]))
    }

    fn write_all(&mut self, bytes: &[u8]) -> DriverResult<()> {
        let port = self.port_mut()?;
        port.write_all(bytes)
            .map_err(|error| io_failure("写入", error))?;
        port.flush().map_err(|error| io_failure("刷新", error))
    }

    fn read_utf8(&mut self, maximum_bytes: usize) -> DriverResult<InvocationOutput> {
        let mut buffer = vec![0_u8; maximum_bytes];
        let bytes_read = self
            .port_mut()?
            .read(&mut buffer)
            .map_err(|error| io_failure("读取", error))?;
        buffer.truncate(bytes_read);

        let text = String::from_utf8(buffer).map_err(|error| DriverError::Failure(format!(
            "串口返回的内容不是有效 UTF-8 文本: {error}"
        )))?;
        Ok(InvocationOutput::new(vec![text]))
    }

    fn port_mut(&mut self) -> DriverResult<&mut (dyn SerialPort + 'static)> {
        self.port
            .as_deref_mut()
            .ok_or(DriverError::NotInitialized)
    }
}

/// 已验证的串口打开参数。
#[derive(Debug, Clone, PartialEq, Eq)]
struct SerialSettings {
    port_name: String,
    baud_rate: u32,
    data_bits: DataBits,
    parity: Parity,
    stop_bits: StopBits,
    flow_control: FlowControl,
    timeout_ms: u64,
}

impl SerialSettings {
    fn from_config(config: &[String]) -> DriverResult<Self> {
        Ok(Self {
            port_name: config_value(config, 0, "端口名称", DEFAULT_PORT_NAME).to_owned(),
            baud_rate: parse_u32(
                config_value(config, 1, "波特率", DEFAULT_BAUD_RATE_TEXT),
                "波特率",
            )?,
            data_bits: parse_data_bits(config_value(config, 2, "数据位", "8"))?,
            parity: parse_parity(config_value(config, 3, "校验位", "None"))?,
            stop_bits: parse_stop_bits(config_value(config, 4, "停止位", "1"))?,
            flow_control: parse_flow_control(config_value(config, 5, "流控", "None"))?,
            timeout_ms: parse_u64(
                config_value(config, 6, "超时毫秒", DEFAULT_TIMEOUT_MS_TEXT),
                "超时毫秒",
            )?,
        })
    }
}

fn config_value<'a>(config: &'a [String], index: usize, label: &str, default: &'a str) -> &'a str {
    let Some(value) = config.get(index).map(String::as_str) else {
        return default;
    };
    let value = value.trim();
    if value.is_empty() {
        return default;
    }

    value
        .strip_prefix(&format!("{label}:"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(value)
}

fn required_input<'a>(input: &'a [String], index: usize, label: &str) -> DriverResult<&'a str> {
    input
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| DriverError::InvalidArgument {
            field: label.to_owned(),
            message: "不能为空".to_owned(),
        })
}

fn maximum_bytes(input: &[String], index: usize) -> DriverResult<usize> {
    let value = input
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_MAXIMUM_BYTES_TEXT);
    let maximum = value
        .trim()
        .parse::<usize>()
        .map_err(|_| invalid_value("最大接收字节数", "必须是正整数"))?;
    if maximum == 0 {
        return Err(invalid_value("最大接收字节数", "必须大于 0"));
    }
    Ok(maximum)
}

fn parse_u32(value: &str, field: &str) -> DriverResult<u32> {
    value
        .trim()
        .parse()
        .map_err(|_| invalid_value(field, "必须是无符号整数"))
}

fn parse_u64(value: &str, field: &str) -> DriverResult<u64> {
    value
        .trim()
        .parse()
        .map_err(|_| invalid_value(field, "必须是无符号整数"))
}

fn parse_data_bits(value: &str) -> DriverResult<DataBits> {
    match value.trim() {
        "5" => Ok(DataBits::Five),
        "6" => Ok(DataBits::Six),
        "7" => Ok(DataBits::Seven),
        "8" => Ok(DataBits::Eight),
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

fn parse_stop_bits(value: &str) -> DriverResult<StopBits> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "one" => Ok(StopBits::One),
        "2" | "two" => Ok(StopBits::Two),
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

fn serial_failure(action: &str, error: serialport::Error) -> DriverError {
    DriverError::Failure(format!("串口{action}失败: {error}"))
}

fn io_failure(action: &str, error: std::io::Error) -> DriverError {
    DriverError::Failure(format!("串口{action}失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 使用默认配置并允许省略可选项() {
        let config = vec!["COM8".to_owned(), "9600".to_owned()];
        let settings = SerialSettings::from_config(&config).unwrap();

        assert_eq!(settings.port_name, "COM8");
        assert_eq!(settings.baud_rate, 9_600);
        assert_eq!(settings.data_bits, DataBits::Eight);
        assert_eq!(settings.timeout_ms, 1_000);
    }

    #[test]
    fn 支持带标签的配置文本() {
        let config = vec![
            "端口名称:COM12".to_owned(),
            "波特率:57600".to_owned(),
            "数据位:7".to_owned(),
            "校验位:偶校验".to_owned(),
            "停止位:2".to_owned(),
            "流控:硬件".to_owned(),
            "超时毫秒:250".to_owned(),
        ];
        let settings = SerialSettings::from_config(&config).unwrap();

        assert_eq!(settings.port_name, "COM12");
        assert_eq!(settings.baud_rate, 57_600);
        assert_eq!(settings.data_bits, DataBits::Seven);
        assert_eq!(settings.parity, Parity::Even);
        assert_eq!(settings.stop_bits, StopBits::Two);
        assert_eq!(settings.flow_control, FlowControl::Hardware);
        assert_eq!(settings.timeout_ms, 250);
    }

    #[test]
    fn 拒绝不支持的串口参数() {
        let error = parse_data_bits("9").unwrap_err();

        assert_eq!(
            error,
            DriverError::InvalidArgument {
                field: "数据位".to_owned(),
                message: "仅支持 5、6、7 或 8".to_owned(),
            }
        );
    }

    #[test]
    fn 接收长度必须为正整数() {
        assert!(maximum_bytes(&["0".to_owned()], 0).is_err());
        assert_eq!(maximum_bytes(&[], 0).unwrap(), 1_024);
    }
}
