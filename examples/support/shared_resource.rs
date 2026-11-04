//! 示例通信提供者：以独占文件句柄验证打开次数和完整事务，未连接真实硬件。
use miresidian::{
    MiresidianError, MiresidianResult,
    lease::{
        LeaseConfiguration, LeaseError, ResourceKey,
        shared::{ResourceHandler, SharedResource},
    },
};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    time::Duration,
};

pub(super) fn error(error: LeaseError) -> MiresidianError {
    MiresidianError::Failure(error.to_string())
}

pub(super) fn connect(path: &str) -> MiresidianResult<SharedResource> {
    let path = std::path::Path::new(path)
        .canonicalize()
        .map_err(|e| MiresidianError::Failure(e.to_string()))?;
    let resource =
        ResourceKey::new(format!("example-exclusive:{}", path.display())).map_err(error)?;
    SharedResource::acquire(
        resource,
        LeaseConfiguration::new(vec![1]).map_err(error)?,
        "miresidian-example-echo/1",
        || {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(io_error)?;
            file.try_lock()
                .map_err(|_| LeaseError::LocalIpc("资源被重复打开".into()))?;
            let mut events = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.with_extension("events"))
                .map_err(io_error)?;
            writeln!(events, "open").map_err(io_error)?;
            Ok(ExclusiveResource {
                _file: file,
                events,
            })
        },
    )
    .map_err(error)
}

pub(super) fn query(
    resource: &SharedResource,
    calls: &mut usize,
    node: &str,
    text: &str,
) -> MiresidianResult<String> {
    let response = resource
        .transact(text.as_bytes(), Duration::from_secs(2))
        .map_err(error)?;
    *calls += 1;
    Ok(format!(
        "{node}:{}:{}",
        calls,
        String::from_utf8(response).map_err(|e| MiresidianError::Failure(e.to_string()))?
    ))
}

struct ExclusiveResource {
    _file: File,
    events: File,
}
impl ResourceHandler for ExclusiveResource {
    fn transact(&mut self, request: &[u8]) -> Result<Vec<u8>, LeaseError> {
        writeln!(self.events, "begin:{}", String::from_utf8_lossy(request)).map_err(io_error)?;
        std::thread::sleep(Duration::from_millis(10));
        writeln!(self.events, "end:{}", String::from_utf8_lossy(request)).map_err(io_error)?;
        Ok(request.to_vec())
    }
}
impl Drop for ExclusiveResource {
    fn drop(&mut self) {
        let _ = writeln!(self.events, "close");
    }
}
fn io_error(error: std::io::Error) -> LeaseError {
    LeaseError::LocalIpc(error.to_string())
}
