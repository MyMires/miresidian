//! 按资源选举唯一所有者。文件锁覆盖监听、物理连接及所有在途事务的整个寿命。
use super::{ResourceHandler, server};
use crate::local_ipc::protocol::io_error;
use crate::local_ipc::{LocalLease, resource::user_scope, service::pin_service_module};
use crate::{LeaseConfiguration, LeaseError, ResourceKey};
use interprocess::{
    ConnectWaitMode,
    local_socket::{
        ConnectOptions, GenericNamespaced, ListenerNonblockingMode, ListenerOptions, Stream,
        ToNsName, prelude::*,
    },
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions, TryLockError},
    thread,
    time::Duration,
};

fn name(resource: &ResourceKey) -> String {
    let mut hash = Sha256::new();
    hash.update(user_scope());
    hash.update([0]);
    hash.update(resource.as_str().as_bytes());
    format!("miresidian-resource-v1-{:x}", hash.finalize())
}

pub(super) fn connect(resource: &ResourceKey) -> Result<Stream, LeaseError> {
    let name = name(resource)
        .to_ns_name::<GenericNamespaced>()
        .map_err(io_error)?;
    ConnectOptions::new()
        .name(name)
        .wait_mode(ConnectWaitMode::Timeout(Duration::from_millis(50)))
        .connect_sync()
        .map_err(io_error)
}

pub(super) fn try_start<F, H>(
    resource: &ResourceKey,
    configuration: &LeaseConfiguration,
    protocol: &str,
    create: &mut Option<F>,
) -> Result<(), LeaseError>
where
    F: FnOnce() -> Result<H, LeaseError>,
    H: ResourceHandler,
{
    let directory = std::env::temp_dir()
        .join("miresidian-resource-v1")
        .join(user_scope());
    fs::create_dir_all(&directory).map_err(io_error)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join(format!(
            "{:x}.owner",
            Sha256::digest(resource.as_str().as_bytes())
        )))
        .map_err(io_error)?;
    match file.try_lock() {
        Err(TryLockError::WouldBlock) => return Ok(()),
        Err(TryLockError::Error(error)) => return Err(io_error(error)),
        Ok(()) => {}
    }
    let name = name(resource)
        .to_ns_name::<GenericNamespaced>()
        .map_err(io_error)?;
    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .map_err(io_error)?;
    listener
        .set_nonblocking(ListenerNonblockingMode::Accept)
        .map_err(io_error)?;
    let lease = LocalLease::acquire(resource.clone(), configuration.clone())?;
    // 工厂和处理器回调位于同一个调用方 DLL；服务存活时不得卸载其代码。
    pin_service_module()?;
    let create = create.take().ok_or(LeaseError::Invalidated)?;
    let handler = std::panic::catch_unwind(std::panic::AssertUnwindSafe(create))
        .map_err(|_| LeaseError::LocalIpc("资源连接工厂发生 panic".into()))??;
    let state = server::OwnerState::new(
        Box::new(handler),
        lease,
        configuration.clone(),
        protocol.into(),
    );
    thread::Builder::new()
        .name("miresidian-resource-owner".into())
        .spawn(move || {
            server::serve(listener, state);
            // listener 在 serve 返回前析构；之后才能允许下一个所有者绑定并打开资源。
            drop(file);
        })
        .map_err(io_error)?;
    Ok(())
}
