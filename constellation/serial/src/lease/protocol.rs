//! 串口租约客户端与代理共用的回环协议。

use std::{
    io::{Read, Write},
    net::TcpStream,
};

use miresidian::{MiresidianError, MiresidianResult};
use serialport::{DataBits, FlowControl, Parity, StopBits};

use crate::transport::SerialSettings;

pub(super) const ACQUIRE: u8 = 1;
pub(super) const WRITE: u8 = 2;
pub(super) const READ: u8 = 3;
pub(super) const QUERY: u8 = 4;
pub(super) const RELEASE: u8 = 5;
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// 发送请求并将代理端的失败文本转换为驱动错误。
pub(super) fn request(
    stream: &mut TcpStream,
    operation: u8,
    payload: &[u8],
) -> MiresidianResult<Vec<u8>> {
    write_frame(stream, operation, payload)
        .map_err(|error| failure(format!("发送串口代理请求失败：{error}")))?;
    let (status, payload) =
        read_frame(stream).map_err(|error| failure(format!("读取串口代理响应失败：{error}")))?;
    if status == 0 {
        Ok(payload)
    } else {
        Err(failure(String::from_utf8_lossy(&payload).into_owned()))
    }
}

/// 写入代理端的成功或失败响应。
pub(super) fn write_response(
    stream: &mut TcpStream,
    response: MiresidianResult<Vec<u8>>,
) -> std::io::Result<()> {
    match response {
        Ok(payload) => write_frame(stream, 0, &payload),
        Err(error) => write_frame(stream, 1, error.to_string().as_bytes()),
    }
}

/// 从字节序列编码通信设置。
pub(super) fn encode_settings(settings: &SerialSettings) -> Vec<u8> {
    let mut payload = Vec::new();
    push_bytes(&mut payload, settings.port_name.as_bytes()).expect("端口名称长度有效");
    payload.extend_from_slice(&settings.baud_rate.to_be_bytes());
    payload.push(data_bits_code(settings.data_bits));
    payload.push(parity_code(settings.parity));
    payload.push(stop_bits_code(settings.stop_bits));
    payload.push(flow_control_code(settings.flow_control));
    payload.extend_from_slice(&settings.timeout_ms.to_be_bytes());
    payload
}

/// 解码客户端提交的通信设置。
pub(super) fn decode_settings(payload: &[u8]) -> MiresidianResult<SerialSettings> {
    let mut offset = 0;
    let port_name = String::from_utf8(read_bytes_field(payload, &mut offset)?.to_vec())
        .map_err(|_| failure("端口名称必须为 UTF-8"))?;
    let baud_rate = read_u32(payload, &mut offset)?;
    let data_bits = decode_serial_data_bits(read_u8(payload, &mut offset)?)?;
    let parity = decode_parity(read_u8(payload, &mut offset)?)?;
    let stop_bits = decode_stop_bits(read_u8(payload, &mut offset)?)?;
    let flow_control = decode_flow_control(read_u8(payload, &mut offset)?)?;
    let timeout_ms = read_u64(payload, &mut offset)?;
    if offset != payload.len() {
        return Err(failure("串口代理配置包含多余内容"));
    }
    Ok(SerialSettings {
        port_name,
        baud_rate,
        data_bits,
        parity,
        stop_bits,
        flow_control,
        timeout_ms,
    })
}

pub(super) fn decode_lease(payload: &[u8]) -> MiresidianResult<[u8; 16]> {
    payload
        .try_into()
        .map_err(|_| failure("串口代理租约标识无效"))
}

pub(super) fn decode_lease_bytes(payload: &[u8]) -> MiresidianResult<([u8; 16], Vec<u8>)> {
    let lease = decode_lease_prefix(payload)?;
    let mut offset = 16;
    Ok((lease, read_bytes_field(payload, &mut offset)?.to_vec()))
}

pub(super) fn decode_query(payload: &[u8]) -> MiresidianResult<([u8; 16], Vec<u8>)> {
    let lease = decode_lease_prefix(payload)?;
    let mut offset = 16;
    let bytes = read_bytes_field(payload, &mut offset)?.to_vec();
    if offset != payload.len() {
        return Err(failure("串口查询请求包含多余内容"));
    }
    Ok((lease, bytes))
}

pub(super) fn push_bytes(target: &mut Vec<u8>, bytes: &[u8]) -> MiresidianResult<()> {
    let length = u32::try_from(bytes.len()).map_err(|_| failure("代理消息过长"))?;
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(bytes);
    Ok(())
}

pub(super) fn read_frame(stream: &mut TcpStream) -> std::io::Result<(u8, Vec<u8>)> {
    let mut operation = [0; 1];
    let mut length = [0; 4];
    stream.read_exact(&mut operation)?;
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_SIZE {
        return Err(std::io::Error::other("帧超过最大长度"));
    }
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload)?;
    Ok((operation[0], payload))
}

fn write_frame(stream: &mut TcpStream, operation: u8, payload: &[u8]) -> std::io::Result<()> {
    let length = u32::try_from(payload.len()).map_err(|_| std::io::Error::other("帧过大"))?;
    stream.write_all(&[operation])?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(payload)?;
    stream.flush()
}

fn decode_lease_prefix(payload: &[u8]) -> MiresidianResult<[u8; 16]> {
    payload
        .get(..16)
        .ok_or_else(|| failure("串口代理请求缺少租约"))?
        .try_into()
        .map_err(|_| failure("串口代理租约无效"))
}

fn read_bytes_field<'a>(payload: &'a [u8], offset: &mut usize) -> MiresidianResult<&'a [u8]> {
    let length = read_u32(payload, offset)? as usize;
    let end = offset
        .checked_add(length)
        .ok_or_else(|| failure("代理消息长度溢出"))?;
    let value = payload
        .get(*offset..end)
        .ok_or_else(|| failure("代理消息长度不足"))?;
    *offset = end;
    Ok(value)
}

fn read_u8(payload: &[u8], offset: &mut usize) -> MiresidianResult<u8> {
    let value = *payload
        .get(*offset)
        .ok_or_else(|| failure("代理消息长度不足"))?;
    *offset += 1;
    Ok(value)
}

fn read_u32(payload: &[u8], offset: &mut usize) -> MiresidianResult<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| failure("代理消息长度溢出"))?;
    let value = payload
        .get(*offset..end)
        .ok_or_else(|| failure("代理消息长度不足"))?
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| failure("代理消息无效"))?;
    *offset = end;
    Ok(value)
}

fn read_u64(payload: &[u8], offset: &mut usize) -> MiresidianResult<u64> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| failure("代理消息长度溢出"))?;
    let value = payload
        .get(*offset..end)
        .ok_or_else(|| failure("代理消息长度不足"))?
        .try_into()
        .map(u64::from_be_bytes)
        .map_err(|_| failure("代理消息无效"))?;
    *offset = end;
    Ok(value)
}

fn data_bits_code(value: DataBits) -> u8 {
    match value {
        DataBits::Five => 5,
        DataBits::Six => 6,
        DataBits::Seven => 7,
        DataBits::Eight => 8,
    }
}
/// 将租约协议中的数据位字段还原为串口库枚举。
///
/// 这是串口代理内部协议字段的解码，不属于框架 `codec` 的文本—字节转换能力。
fn decode_serial_data_bits(value: u8) -> MiresidianResult<DataBits> {
    match value {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        _ => Err(failure("代理数据位无效")),
    }
}
fn parity_code(value: Parity) -> u8 {
    match value {
        Parity::None => 0,
        Parity::Odd => 1,
        Parity::Even => 2,
    }
}
fn decode_parity(value: u8) -> MiresidianResult<Parity> {
    match value {
        0 => Ok(Parity::None),
        1 => Ok(Parity::Odd),
        2 => Ok(Parity::Even),
        _ => Err(failure("代理校验位无效")),
    }
}
fn stop_bits_code(value: StopBits) -> u8 {
    match value {
        StopBits::One => 1,
        StopBits::Two => 2,
    }
}
fn decode_stop_bits(value: u8) -> MiresidianResult<StopBits> {
    match value {
        1 => Ok(StopBits::One),
        2 => Ok(StopBits::Two),
        _ => Err(failure("代理停止位无效")),
    }
}
fn flow_control_code(value: FlowControl) -> u8 {
    match value {
        FlowControl::None => 0,
        FlowControl::Software => 1,
        FlowControl::Hardware => 2,
    }
}
fn decode_flow_control(value: u8) -> MiresidianResult<FlowControl> {
    match value {
        0 => Ok(FlowControl::None),
        1 => Ok(FlowControl::Software),
        2 => Ok(FlowControl::Hardware),
        _ => Err(failure("代理流控无效")),
    }
}

/// 构造可透传到调用方的协议或代理错误。
pub(super) fn failure(message: impl Into<String>) -> MiresidianError {
    MiresidianError::Failure(message.into())
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, thread};

    use super::*;

    fn settings() -> SerialSettings {
        SerialSettings {
            port_name: "COM7".to_owned(),
            baud_rate: 115_200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            timeout_ms: 800,
        }
    }

    #[test]
    fn 通信设置可无损往返编码() {
        let original = settings();
        assert_eq!(
            decode_settings(&encode_settings(&original)).unwrap(),
            original
        );
    }

    #[test]
    fn 截断的租约协议报错而不恐慌() {
        assert!(decode_settings(&[0, 0, 0]).is_err());
        assert!(decode_lease(&[0; 15]).is_err());
        assert!(decode_query(&[0; 16]).is_err());
    }

    #[test]
    fn 回环代理响应可被客户端正确解析() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let (_, payload) = read_frame(&mut stream).unwrap();
            assert_eq!(payload, "请求".as_bytes());
            write_response(&mut stream, Ok("响应".as_bytes().to_vec())).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();
        assert_eq!(
            request(&mut client, ACQUIRE, "请求".as_bytes()).unwrap(),
            "响应".as_bytes()
        );
        server.join().unwrap();
    }

    #[test]
    fn 代理失败文本会转换为驱动错误() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_frame(&mut stream).unwrap();
            write_response(&mut stream, Err(failure("租约不匹配"))).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();
        assert!(
            request(&mut client, ACQUIRE, &[])
                .unwrap_err()
                .to_string()
                .contains("租约不匹配")
        );
        server.join().unwrap();
    }
}
