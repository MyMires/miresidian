//! 串口租约客户端与代理共用的回环协议。

use std::{
    io::{Read, Write},
    net::TcpStream,
};

use miresidian::{DriverError, DriverResult};
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
) -> DriverResult<Vec<u8>> {
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
    response: DriverResult<Vec<u8>>,
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
pub(super) fn decode_settings(payload: &[u8]) -> DriverResult<SerialSettings> {
    let mut offset = 0;
    let port_name = String::from_utf8(read_bytes_field(payload, &mut offset)?.to_vec())
        .map_err(|_| failure("端口名称必须为 UTF-8"))?;
    let baud_rate = read_u32(payload, &mut offset)?;
    let data_bits = decode_data_bits(read_u8(payload, &mut offset)?)?;
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

pub(super) fn decode_lease(payload: &[u8]) -> DriverResult<[u8; 16]> {
    payload
        .try_into()
        .map_err(|_| failure("串口代理租约标识无效"))
}

pub(super) fn decode_lease_bytes(payload: &[u8]) -> DriverResult<([u8; 16], Vec<u8>)> {
    let lease = decode_lease_prefix(payload)?;
    let mut offset = 16;
    Ok((lease, read_bytes_field(payload, &mut offset)?.to_vec()))
}

pub(super) fn decode_lease_length(payload: &[u8]) -> DriverResult<([u8; 16], usize)> {
    let lease = decode_lease_prefix(payload)?;
    let mut offset = 16;
    Ok((lease, read_u32(payload, &mut offset)? as usize))
}

pub(super) fn decode_query(payload: &[u8]) -> DriverResult<([u8; 16], Vec<u8>, usize)> {
    let lease = decode_lease_prefix(payload)?;
    let mut offset = 16;
    let bytes = read_bytes_field(payload, &mut offset)?.to_vec();
    Ok((lease, bytes, read_u32(payload, &mut offset)? as usize))
}

pub(super) fn push_bytes(target: &mut Vec<u8>, bytes: &[u8]) -> DriverResult<()> {
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

fn decode_lease_prefix(payload: &[u8]) -> DriverResult<[u8; 16]> {
    payload
        .get(..16)
        .ok_or_else(|| failure("串口代理请求缺少租约"))?
        .try_into()
        .map_err(|_| failure("串口代理租约无效"))
}

fn read_bytes_field<'a>(payload: &'a [u8], offset: &mut usize) -> DriverResult<&'a [u8]> {
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

fn read_u8(payload: &[u8], offset: &mut usize) -> DriverResult<u8> {
    let value = *payload
        .get(*offset)
        .ok_or_else(|| failure("代理消息长度不足"))?;
    *offset += 1;
    Ok(value)
}

fn read_u32(payload: &[u8], offset: &mut usize) -> DriverResult<u32> {
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

fn read_u64(payload: &[u8], offset: &mut usize) -> DriverResult<u64> {
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
fn decode_data_bits(value: u8) -> DriverResult<DataBits> {
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
fn decode_parity(value: u8) -> DriverResult<Parity> {
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
fn decode_stop_bits(value: u8) -> DriverResult<StopBits> {
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
fn decode_flow_control(value: u8) -> DriverResult<FlowControl> {
    match value {
        0 => Ok(FlowControl::None),
        1 => Ok(FlowControl::Software),
        2 => Ok(FlowControl::Hardware),
        _ => Err(failure("代理流控无效")),
    }
}

/// 构造可透传到调用方的协议或代理错误。
pub(super) fn failure(message: impl Into<String>) -> DriverError {
    DriverError::Failure(message.into())
}
