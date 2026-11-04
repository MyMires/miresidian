//! 资源代理的有界握手及事务帧；无裸指针、Rust 对象布局或 DLL 文件名。
use crate::local_ipc::protocol::{read_bytes, read_u8, write_bytes, write_u8};
use crate::{LeaseError, MAX_LEASE_CONFIGURATION_BYTES};
use std::io::{Read, Write};

pub(super) const CALL: u8 = 1;
pub(super) const CLOSE: u8 = 2;

pub(super) fn write_result(
    stream: &mut impl Write,
    result: Result<&[u8], &LeaseError>,
) -> Result<(), LeaseError> {
    match result {
        Ok(bytes) => {
            write_u8(stream, 0)?;
            write_bytes(stream, bytes)
        }
        Err(error) => {
            let code = match error {
                LeaseError::IncompatibleConfiguration => 2,
                LeaseError::Invalidated => 3,
                _ => 1,
            };
            write_u8(stream, code)?;
            let message = error.to_string();
            // 错误帧也有大小上限；在 UTF-8 字符边界截断诊断。
            let end = message
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= MAX_LEASE_CONFIGURATION_BYTES)
                .last()
                .unwrap_or(0);
            let message = if message.len() > MAX_LEASE_CONFIGURATION_BYTES {
                &message[..end]
            } else {
                &message
            };
            write_bytes(stream, message.as_bytes())
        }
    }
}

/// 外层错误是协议/连接失效，内层错误是已收到完整响应的业务失败。
pub(super) fn read_result(
    stream: &mut impl Read,
) -> Result<Result<Vec<u8>, LeaseError>, LeaseError> {
    let code = read_u8(stream)?;
    let bytes = read_bytes(
        stream,
        if code == 0 {
            super::MAX_TRANSACTION_BYTES
        } else {
            MAX_LEASE_CONFIGURATION_BYTES
        },
    )?;
    Ok(match code {
        0 => Ok(bytes),
        1 => Err(LeaseError::LocalIpc(
            String::from_utf8_lossy(&bytes).into_owned(),
        )),
        2 => Err(LeaseError::IncompatibleConfiguration),
        3 => Err(LeaseError::Invalidated),
        _ => return Err(LeaseError::Invalidated),
    })
}
