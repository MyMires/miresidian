//! Shared executor and cooperative request context.
//!
//! Tokio 1.53.3 LTS (MIT, MSRV 1.71) supplies maintained cross-platform scheduling,
//! timers and I/O. futures-util supplies panic-safe Future polling. Blocking
//! business runs on Tokio's bounded blocking pool; native futures never use it.
//! See <https://docs.rs/tokio/1.53.3/tokio/runtime/> and task/spawn_blocking.
mod context;
use crate::{MiresidianError, MiresidianResult};
pub use context::CallContext;
use std::{
    future::Future,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::runtime::{Builder, Runtime};

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
pub(crate) struct Activity;
impl Activity {
    pub(crate) fn new() -> Self {
        ACTIVE.fetch_add(1, Ordering::AcqRel);
        Self
    }
}
impl Drop for Activity {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

static LIFECYCLE_EXECUTOR: Mutex<Option<Arc<Runtime>>> = Mutex::new(None);
static EXECUTOR: Mutex<Option<Arc<Runtime>>> = Mutex::new(None);

pub(crate) fn executor() -> MiresidianResult<Arc<Runtime>> {
    let mut slot = EXECUTOR
        .lock()
        .map_err(|_| MiresidianError::Internal("执行器锁污染".into()))?;
    if let Some(runtime) = slot.as_ref() {
        return Ok(runtime.clone());
    }
    let runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(32)
        .thread_name("miresidian")
        .enable_all()
        .build()
        .map_err(|e| MiresidianError::Internal(e.to_string()))?;
    let runtime = Arc::new(runtime);
    *slot = Some(runtime.clone());
    Ok(runtime)
}

pub(crate) fn lifecycle_executor() -> MiresidianResult<Arc<Runtime>> {
    let mut slot = LIFECYCLE_EXECUTOR
        .lock()
        .map_err(|_| MiresidianError::Internal("生命周期执行器锁污染".into()))?;
    if let Some(runtime) = slot.as_ref() {
        return Ok(runtime.clone());
    }
    let runtime = Arc::new(
        Builder::new_multi_thread()
            .worker_threads(1)
            .max_blocking_threads(4)
            .thread_name("miresidian-life")
            .enable_all()
            .build()
            .map_err(|e| MiresidianError::Internal(e.to_string()))?,
    );
    *slot = Some(runtime.clone());
    Ok(runtime)
}

/// Wait for a framework future from synchronous code. Inside async code use
/// the corresponding async runtime method instead of nesting an executor.
pub fn block_on<T>(future: impl Future<Output = MiresidianResult<T>>) -> MiresidianResult<T> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            return Err(MiresidianError::NotSupported(
                "单线程异步上下文请使用 invoke_async".into(),
            ));
        }
        tokio::task::block_in_place(|| executor()?.block_on(future))
    } else {
        executor()?.block_on(future)
    }
}

/// Stop and join the shared executor after all task services have drained.
/// Returns an error while synchronous callers still hold an executor reference.
/// Must be called outside an async executor and before unloading the DLL.
pub fn shutdown_executor() -> MiresidianResult<()> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(MiresidianError::NotSupported(
            "执行器须在同步上下文关闭".into(),
        ));
    }
    if ACTIVE.load(Ordering::Acquire) > 0 {
        return Err(MiresidianError::Failure("执行器仍有在途任务".into()));
    }
    let runtime = {
        let mut slot = EXECUTOR
            .lock()
            .map_err(|_| MiresidianError::Internal("执行器锁污染".into()))?;
        if slot.as_ref().is_some_and(|r| Arc::strong_count(r) != 1) {
            return Err(MiresidianError::Failure("执行器仍有在途调用".into()));
        }
        slot.take()
    };
    if let Some(runtime) = runtime {
        drop(runtime);
    }
    let life = LIFECYCLE_EXECUTOR
        .lock()
        .map_err(|_| MiresidianError::Internal("生命周期执行器锁污染".into()))?
        .take();
    drop(life);
    Ok(())
}

/// Limits apply to one runtime. Pending requests acquire instance execution
/// rights before global running permits, so one busy instance cannot occupy
/// all blocking workers while waiting for its own lock.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimits {
    pub max_running: usize,
    pub max_pending: usize,
    pub max_pending_per_instance: usize,
    pub max_batch: usize,
}
impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_running: 32,
            max_pending: 1024,
            max_pending_per_instance: 64,
            max_batch: 256,
        }
    }
}
impl ExecutionLimits {
    pub(crate) fn validate(self) -> MiresidianResult<Self> {
        if [
            self.max_running,
            self.max_pending,
            self.max_pending_per_instance,
            self.max_batch,
        ]
        .contains(&0)
            || self.max_running > 65536
            || self.max_pending > 65536
            || self.max_pending_per_instance > 65536
            || self.max_batch > 65536
        {
            return Err(MiresidianError::InvalidArgument {
                field: "ExecutionLimits".into(),
                message: "限制须为正数；所有限制 <= 65536".into(),
            });
        }
        Ok(self)
    }
}

/// A compatibility source for generated and existing Rust FFI containers.
#[doc(hidden)]
pub trait RuntimeSource<N: crate::DeviceNode>: Sync {
    fn runtime(
        &self,
        platforms: impl FnOnce() -> crate::PlatformRegistry,
    ) -> MiresidianResult<crate::NodeRuntime<N>>;
}
impl<N: crate::DeviceNode> RuntimeSource<N> for OnceLock<crate::NodeRuntime<N>> {
    fn runtime(
        &self,
        platforms: impl FnOnce() -> crate::PlatformRegistry,
    ) -> MiresidianResult<crate::NodeRuntime<N>> {
        Ok(self
            .get_or_init(|| crate::NodeRuntime::with_platforms(platforms()))
            .clone())
    }
}
impl<N: crate::DeviceNode> RuntimeSource<N> for OnceLock<Mutex<crate::NodeRuntime<N>>> {
    fn runtime(
        &self,
        platforms: impl FnOnce() -> crate::PlatformRegistry,
    ) -> MiresidianResult<crate::NodeRuntime<N>> {
        let mutex =
            self.get_or_init(|| Mutex::new(crate::NodeRuntime::with_platforms(platforms())));
        let runtime = match mutex.lock() {
            Ok(r) => r,
            Err(e) => {
                let r = e.into_inner();
                let _ = r.shutdown();
                mutex.clear_poison();
                r
            }
        };
        Ok(runtime.clone())
    }
}
