//! 客户端持有的跨进程配置锁；服务进程退出不会释放其他客户端的锁。
//!
//! 使用 Rust 1.89 的标准库文件锁；SHA-256 仅用于将资源键映射为固定长度文件名。
//! 锁文件永久保留以维持 inode/文件身份，资源释放依赖操作系统关闭文件句柄。
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Seek, SeekFrom, Write},
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};

use super::protocol::io_error;
use crate::{LeaseConfiguration, LeaseError, MAX_LEASE_CONFIGURATION_BYTES, ResourceKey};

pub(super) struct ResourceGuard {
    _file: File,
}

pub(super) fn user_scope() -> String {
    let scope = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .or_else(|| std::env::var_os("USERNAME"))
        .or_else(|| std::env::var_os("USER"))
        .unwrap_or_default();
    format!("{:x}", Sha256::digest(scope.as_encoded_bytes()))
}

impl ResourceGuard {
    pub(super) fn acquire(
        resource: &ResourceKey,
        configuration: &LeaseConfiguration,
    ) -> Result<Self, LeaseError> {
        let directory = std::env::temp_dir()
            .join("miresidian-lease-v2")
            .join(user_scope());
        fs::create_dir_all(&directory).map_err(io_error)?;
        let name = format!("{:x}.lock", Sha256::digest(resource.as_str().as_bytes()));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join(name))
            .map_err(io_error)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match file.try_lock() {
                Ok(()) => {
                    file.set_len(0).map_err(io_error)?;
                    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
                    file.write_all(configuration.as_bytes()).map_err(io_error)?;
                    file.flush().map_err(io_error)?;
                    // 先解独占锁，再取共享锁；间隙中若被改写，下面的配置校验会拒绝。
                    file.unlock().map_err(io_error)?;
                }
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(error)) => return Err(io_error(error)),
            }
            match file.try_lock_shared() {
                Ok(()) => {
                    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
                    let mut actual = Vec::new();
                    (&mut file)
                        .take(MAX_LEASE_CONFIGURATION_BYTES as u64 + 1)
                        .read_to_end(&mut actual)
                        .map_err(io_error)?;
                    if actual != configuration.as_bytes() {
                        return Err(LeaseError::IncompatibleConfiguration);
                    }
                    return Ok(Self { _file: file });
                }
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(LeaseError::LocalIpc("等待资源配置锁超时".to_owned()));
                }
                Err(TryLockError::Error(error)) => return Err(io_error(error)),
            }
        }
    }
}
