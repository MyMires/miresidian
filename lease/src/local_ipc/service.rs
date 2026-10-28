//! 本地租约协调服务；资源配置文件锁由客户端持有，独立于服务进程寿命。
use super::{protocol::*, resource::user_scope};
use crate::{LeaseError, LeaseId, LeaseRegistry};
use interprocess::ConnectWaitMode;
use interprocess::local_socket::{
    ConnectOptions, GenericNamespaced, Listener, ListenerOptions, Stream, ToNsName, prelude::*,
};
use std::{
    io,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

fn service_name() -> String {
    format!("miresidian-lease-v2-{}", user_scope())
}

pub(super) fn connect_or_start() -> Result<Stream, LeaseError> {
    if let Ok(stream) = connect() {
        return Ok(stream);
    }

    let name = service_name()
        .to_ns_name::<GenericNamespaced>()
        .map_err(io_error)?;
    match ListenerOptions::new().name(name).create_sync() {
        Ok(listener) => {
            pin_service_module()?;
            let generation = LeaseId::random()?;
            thread::Builder::new()
                .name("miresidian-lease-service".to_owned())
                .spawn(move || serve(listener, generation))
                .map_err(|error| LeaseError::LocalIpc(format!("无法启动本地租约服务：{error}")))?;
        }
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {}
        Err(error) => return Err(io_error(error)),
    }

    for _ in 0..50 {
        if let Ok(stream) = connect() {
            return Ok(stream);
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(LeaseError::LocalIpc(
        "无法连接 Miresidian 本地租约服务".to_owned(),
    ))
}

fn connect() -> Result<Stream, LeaseError> {
    let name = service_name()
        .to_ns_name::<GenericNamespaced>()
        .map_err(io_error)?;
    ConnectOptions::new()
        .name(name)
        .wait_mode(ConnectWaitMode::Timeout(Duration::from_millis(500)))
        .connect_sync()
        .map_err(io_error)
}

/// 服务线程执行框架代码，因此 Windows 上固定首个承载 DLL，直至进程退出。
#[cfg(windows)]
fn pin_service_module() -> Result<(), LeaseError> {
    unsafe extern "system" {
        fn GetModuleHandleExW(
            flags: u32,
            module_name: *const u16,
            module: *mut *mut core::ffi::c_void,
        ) -> i32;
    }

    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
    const GET_MODULE_HANDLE_EX_FLAG_PIN: u32 = 0x0000_0001;
    let mut module = core::ptr::null_mut();
    // SAFETY: FROM_ADDRESS 使用当前模块内的函数地址，输出参数是有效局部指针。
    let succeeded = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
            pin_service_module as *const () as *const u16,
            &mut module,
        )
    };
    if succeeded == 0 || module.is_null() {
        return Err(LeaseError::LocalIpc(
            "无法固定本地租约服务宿主 DLL".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn pin_service_module() -> Result<(), LeaseError> {
    Ok(())
}

fn serve(listener: Listener, generation: LeaseId) {
    let registry = Arc::new(Mutex::new(LeaseRegistry::default()));
    for stream in listener.incoming().flatten() {
        let registry = Arc::clone(&registry);
        let _ = thread::Builder::new()
            .name("miresidian-lease-client".to_owned())
            .spawn(move || serve_client(stream, registry, generation));
    }
}

fn serve_client(mut stream: Stream, registry: Arc<Mutex<LeaseRegistry>>, generation: LeaseId) {
    let grant = read_acquire(&mut stream).and_then(|(resource, configuration)| {
        let grant = registry
            .lock()
            .map_err(|_| LeaseError::Invalidated)?
            .acquire(resource.clone(), configuration)?;
        Ok((resource, grant))
    });
    let (resource, grant) = match grant {
        Ok(grant) => grant,
        Err(error) => {
            let _ = write_error(&mut stream, error);
            return;
        }
    };
    if write_grant(&mut stream, &grant, generation).is_err() {
        let _ = release(&registry, grant.lease);
        return;
    }
    while let Ok(action) = read_u8(&mut stream) {
        if action != RELEASE && action != VALIDATE {
            let _ = write_error(&mut stream, LeaseError::Invalidated);
            break;
        }
        let session = read_session(&mut stream);
        if !matches!(session, Ok((id, epoch)) if id == grant.lease && epoch == generation) {
            let _ = write_error(&mut stream, LeaseError::Invalidated);
            break;
        }
        if action == RELEASE {
            let response = release(&registry, grant.lease);
            let _ = match response {
                Ok(()) => write_empty_response(&mut stream),
                Err(error) => write_error(&mut stream, error),
            };
            return;
        }
        let valid = registry
            .lock()
            .is_ok_and(|registry| registry.validate(&resource, grant.lease));
        if !valid || write_empty_response(&mut stream).is_err() {
            break;
        }
    }
    let _ = release(&registry, grant.lease);
}

fn release(registry: &Mutex<LeaseRegistry>, lease: LeaseId) -> Result<(), LeaseError> {
    registry
        .lock()
        .map_err(|_| LeaseError::Invalidated)?
        .release(lease)
        .map(|_| ())
}
