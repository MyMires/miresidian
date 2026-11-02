//! 租约 v2 的有界帧编解码；服务代次与租约 ID 共同标识会话。
use crate::{
    LEASE_PROTOCOL_VERSION, LeaseConfiguration, LeaseError, LeaseGrant, LeaseId, LeaseState,
    MAX_LEASE_CONFIGURATION_BYTES, MAX_RESOURCE_KEY_BYTES, ResourceKey,
};
use std::io::{Read, Write};

pub(super) const ACQUIRE: u8 = 1;
pub(super) const RELEASE: u8 = 2;
pub(super) const VALIDATE: u8 = 3;
const RESPONSE_OK: u8 = 0;
const RESPONSE_ERROR: u8 = 1;

pub(super) fn read_acquire(
    stream: &mut impl Read,
) -> Result<(ResourceKey, LeaseConfiguration), LeaseError> {
    if read_u8(stream)? != ACQUIRE {
        return Err(LeaseError::LocalIpc("本地租约请求类型无效".to_owned()));
    }
    let mut version = [0; 2];
    stream.read_exact(&mut version).map_err(io_error)?;
    if u16::from_le_bytes(version) != LEASE_PROTOCOL_VERSION {
        return Err(LeaseError::LocalIpc("本地租约协议版本不兼容".to_owned()));
    }
    let resource = String::from_utf8(read_bytes(stream, MAX_RESOURCE_KEY_BYTES)?)
        .map_err(|_| LeaseError::LocalIpc("资源键不是有效 UTF-8".to_owned()))?;
    Ok((
        ResourceKey::new(resource)?,
        LeaseConfiguration::new(read_bytes(stream, MAX_LEASE_CONFIGURATION_BYTES)?)?,
    ))
}

pub(super) fn read_grant(stream: &mut impl Read) -> Result<(LeaseGrant, LeaseId), LeaseError> {
    read_status(stream)?;
    let mut id = [0; 32];
    stream.read_exact(&mut id).map_err(io_error)?;
    let state = match read_u8(stream)? {
        0 => LeaseState::First,
        1 => LeaseState::Shared,
        _ => return Err(LeaseError::LocalIpc("本地租约状态无效".to_owned())),
    };
    let mut generation = [0; 32];
    stream.read_exact(&mut generation).map_err(io_error)?;
    Ok((
        LeaseGrant {
            lease: LeaseId(id),
            state,
        },
        LeaseId(generation),
    ))
}

pub(super) fn write_grant(
    stream: &mut impl Write,
    grant: &LeaseGrant,
    generation: LeaseId,
) -> Result<(), LeaseError> {
    write_u8(stream, RESPONSE_OK)?;
    stream.write_all(grant.lease.as_bytes()).map_err(io_error)?;
    write_u8(
        stream,
        match grant.state {
            LeaseState::First => 0,
            LeaseState::Shared => 1,
        },
    )?;
    stream.write_all(generation.as_bytes()).map_err(io_error)
}

pub(super) fn read_empty_response(stream: &mut impl Read) -> Result<(), LeaseError> {
    read_status(stream)
}

pub(super) fn write_empty_response(stream: &mut impl Write) -> Result<(), LeaseError> {
    write_u8(stream, RESPONSE_OK)
}

pub(super) fn read_status(stream: &mut impl Read) -> Result<(), LeaseError> {
    match read_u8(stream)? {
        RESPONSE_OK => Ok(()),
        RESPONSE_ERROR => Err(LeaseError::LocalIpc(
            String::from_utf8_lossy(&read_bytes(stream, MAX_LEASE_CONFIGURATION_BYTES)?)
                .into_owned(),
        )),
        _ => Err(LeaseError::LocalIpc("本地租约响应无效".to_owned())),
    }
}

pub(super) fn write_error(stream: &mut impl Write, error: LeaseError) -> Result<(), LeaseError> {
    write_u8(stream, RESPONSE_ERROR)?;
    write_bytes(stream, error.to_string().as_bytes())
}

pub(super) fn read_u8(stream: &mut impl Read) -> Result<u8, LeaseError> {
    let mut value = [0; 1];
    stream.read_exact(&mut value).map_err(io_error)?;
    Ok(value[0])
}

pub(super) fn write_u8(stream: &mut impl Write, value: u8) -> Result<(), LeaseError> {
    stream.write_all(&[value]).map_err(io_error)
}

pub(super) fn read_bytes(stream: &mut impl Read, maximum: usize) -> Result<Vec<u8>, LeaseError> {
    let mut length = [0; 4];
    stream.read_exact(&mut length).map_err(io_error)?;
    let length = usize::try_from(u32::from_le_bytes(length))
        .map_err(|_| LeaseError::ConfigurationTooLarge)?;
    if length > maximum {
        return Err(LeaseError::ConfigurationTooLarge);
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(io_error)?;
    Ok(bytes)
}

pub(super) fn write_bytes(stream: &mut impl Write, bytes: &[u8]) -> Result<(), LeaseError> {
    let length = u32::try_from(bytes.len()).map_err(|_| LeaseError::ConfigurationTooLarge)?;
    stream.write_all(&length.to_le_bytes()).map_err(io_error)?;
    stream.write_all(bytes).map_err(io_error)
}

pub(super) fn write_session(
    stream: &mut impl Write,
    id: LeaseId,
    generation: LeaseId,
) -> Result<(), LeaseError> {
    stream.write_all(id.as_bytes()).map_err(io_error)?;
    stream.write_all(generation.as_bytes()).map_err(io_error)
}

pub(super) fn read_session(stream: &mut impl Read) -> Result<(LeaseId, LeaseId), LeaseError> {
    let mut id = [0; 32];
    let mut generation = [0; 32];
    stream.read_exact(&mut id).map_err(io_error)?;
    stream.read_exact(&mut generation).map_err(io_error)?;
    Ok((LeaseId(id), LeaseId(generation)))
}

pub(super) fn io_error(error: std::io::Error) -> LeaseError {
    LeaseError::LocalIpc(format!("本地租约 IPC 失败：{error}"))
}
