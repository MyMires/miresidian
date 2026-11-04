//! 连接登记与资源事务。每个资源一把锁，既覆盖完整 I/O，又保护最后释放和新附着的竞争。
use super::{MAX_TRANSACTION_BYTES, RESOURCE_PROTOCOL_VERSION, ResourceHandler, protocol::*};
use crate::local_ipc::{
    LocalLease,
    protocol::{read_bytes, read_u8},
};
use crate::{
    LeaseConfiguration, LeaseError, MAX_LEASE_CONFIGURATION_BYTES, MAX_RESOURCE_KEY_BYTES,
};
use interprocess::local_socket::{Listener, Stream, prelude::*};
use std::{
    io::{self, Read},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub(super) struct OwnerState {
    handler: Option<Box<dyn ResourceHandler>>,
    lease: Option<LocalLease>,
    configuration: LeaseConfiguration,
    protocol: String,
    users: usize,
    accepting: bool,
    faulted: bool,
}

impl OwnerState {
    pub(super) fn new(
        handler: Box<dyn ResourceHandler>,
        lease: LocalLease,
        configuration: LeaseConfiguration,
        protocol: String,
    ) -> Self {
        Self {
            handler: Some(handler),
            lease: Some(lease),
            configuration,
            protocol,
            users: 0,
            accepting: true,
            faulted: false,
        }
    }

    fn attach(&mut self, protocol: &[u8], configuration: &[u8]) -> Result<(), LeaseError> {
        if !self.accepting {
            return Err(LeaseError::Invalidated);
        }
        if self.faulted {
            return Err(LeaseError::LocalIpc(
                "资源所有者已故障，请关闭全部代理后重新初始化".into(),
            ));
        }
        if self.configuration.as_bytes() != configuration {
            return Err(LeaseError::IncompatibleConfiguration);
        }
        if self.protocol.as_bytes() != protocol {
            return Err(LeaseError::LocalIpc("同一资源的通信事务协议不兼容".into()));
        }
        self.lease
            .as_ref()
            .ok_or(LeaseError::Invalidated)?
            .validate()?;
        self.users += 1;
        Ok(())
    }

    fn transact(&mut self, request: &[u8]) -> Result<Vec<u8>, LeaseError> {
        if self.faulted || self.handler.is_none() {
            return Err(LeaseError::Invalidated);
        }
        if self
            .lease
            .as_ref()
            .ok_or(LeaseError::Invalidated)?
            .validate()
            .is_err()
        {
            self.faulted = true;
            return Err(LeaseError::Invalidated);
        }
        match catch_unwind(AssertUnwindSafe(|| {
            self.handler.as_mut().unwrap().transact(request)
        })) {
            Ok(Ok(output)) if output.len() > MAX_TRANSACTION_BYTES => {
                Err(LeaseError::ConfigurationTooLarge)
            }
            Ok(result) => result,
            Err(_) => {
                self.faulted = true;
                Err(LeaseError::Invalidated)
            }
        }
    }

    fn cleanup(&mut self) {
        self.accepting = false;
        if let Some(handler) = self.handler.take() {
            let _ = catch_unwind(AssertUnwindSafe(|| drop(handler)));
        }
        // 物理连接的析构必须早于所有者的配置租约释放。
        if let Some(mut lease) = self.lease.take() {
            let _ = lease.release();
        }
    }
}

impl Drop for OwnerState {
    fn drop(&mut self) {
        self.cleanup();
    }
}

struct Session(Arc<Mutex<OwnerState>>);
impl Drop for Session {
    fn drop(&mut self) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.users -= 1;
        if state.users == 0 {
            state.cleanup();
        }
    }
}

pub(super) fn serve(listener: Listener, state: OwnerState) {
    let state = Arc::new(Mutex::new(state));
    let initial_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        {
            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
            if !state.accepting {
                break;
            }
            if state.users == 0 && Instant::now() >= initial_deadline {
                state.cleanup();
                break;
            }
        }
        match listener.accept() {
            Ok(stream) => {
                let state = state.clone();
                let _ = thread::Builder::new()
                    .name("miresidian-resource-client".into())
                    .spawn(move || serve_client(stream, state));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(_) => {
                // 已接受会话仍持有 state；监听失败不提前关闭在途资源。
                state.lock().unwrap_or_else(|e| e.into_inner()).accepting = false;
                break;
            }
        }
    }
    // 所有者锁在调用方继续持有；必须等全部会话结束才能退出并释放锁。
    while state.lock().unwrap_or_else(|e| e.into_inner()).users != 0 {
        thread::sleep(Duration::from_millis(2));
    }
}

fn serve_client(mut stream: Stream, state: Arc<Mutex<OwnerState>>) {
    let attached = (|| {
        let mut version = [0; 2];
        stream
            .read_exact(&mut version)
            .map_err(crate::local_ipc::protocol::io_error)?;
        if u16::from_le_bytes(version) != RESOURCE_PROTOCOL_VERSION {
            return Err(LeaseError::Invalidated);
        }
        let protocol = read_bytes(&mut stream, MAX_RESOURCE_KEY_BYTES)?;
        let configuration = read_bytes(&mut stream, MAX_LEASE_CONFIGURATION_BYTES)?;
        state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .attach(&protocol, &configuration)
    })();
    if let Err(error) = attached {
        let _ = write_result(&mut stream, Err(&error));
        return;
    }
    let session = Session(state.clone());
    if write_result(&mut stream, Ok(&[])).is_err() {
        return;
    }
    while let Ok(command) = read_u8(&mut stream) {
        match command {
            CALL => {
                let Ok(request) = read_bytes(&mut stream, MAX_TRANSACTION_BYTES) else {
                    return;
                };
                let result = state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .transact(&request);
                if write_result(&mut stream, result.as_deref()).is_err() {
                    return;
                }
            }
            CLOSE => {
                drop(session);
                let _ = write_result(&mut stream, Ok(&[]));
                return;
            }
            _ => return,
        }
    }
}
