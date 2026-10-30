//! Executors own a module reference and join their threads before releasing it.
//! Loader acquisition must precede thread creation, including builder failures.
use super::module::ModuleGuard;
use crate::{MiresidianError, MiresidianResult};
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
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

pub(crate) struct Executor {
    // Rust drops fields in declaration order: join Runtime before module release.
    runtime: Runtime,
    _module: ModuleGuard,
}
impl Executor {
    fn new(lifecycle: bool) -> MiresidianResult<Self> {
        let module = ModuleGuard::retain()?;
        let runtime = Builder::new_multi_thread()
            .worker_threads(if lifecycle { 1 } else { 2 })
            .max_blocking_threads(if lifecycle { 4 } else { 32 })
            .thread_name(if lifecycle {
                "miresidian-life"
            } else {
                "miresidian"
            })
            .enable_all()
            .build()
            .map_err(|e| MiresidianError::Internal(e.to_string()))?;
        Ok(Self {
            runtime,
            _module: module,
        })
    }
    pub(crate) fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let activity = Activity::new();
        self.runtime.spawn(async move {
            let _activity = activity;
            future.await
        })
    }
    pub(crate) fn spawn_blocking<F, T>(&self, call: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let activity = Activity::new();
        self.runtime.spawn_blocking(move || {
            let _activity = activity;
            call()
        })
    }
}
struct Executors {
    business: Option<Arc<Executor>>,
    lifecycle: Option<Arc<Executor>>,
    stopping: bool,
}
static EXECUTORS: Mutex<Executors> = Mutex::new(Executors {
    business: None,
    lifecycle: None,
    stopping: false,
});
fn get(lifecycle: bool) -> MiresidianResult<Arc<Executor>> {
    let mut pools = EXECUTORS
        .lock()
        .map_err(|_| MiresidianError::Internal("执行器锁污染".into()))?;
    if pools.stopping {
        return Err(MiresidianError::Failure("执行器正在关闭".into()));
    }
    let slot = if lifecycle {
        &mut pools.lifecycle
    } else {
        &mut pools.business
    };
    if let Some(runtime) = slot {
        return Ok(runtime.clone());
    }
    let runtime = Arc::new(Executor::new(lifecycle)?);
    *slot = Some(runtime.clone());
    Ok(runtime)
}
pub(crate) fn executor() -> MiresidianResult<Arc<Executor>> {
    get(false)
}
pub(crate) fn lifecycle_executor() -> MiresidianResult<Arc<Executor>> {
    get(true)
}

/// Wait from synchronous code; use the async API inside a single-thread runtime.
pub fn block_on<T>(future: impl Future<Output = MiresidianResult<T>>) -> MiresidianResult<T> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            return Err(MiresidianError::NotSupported(
                "单线程异步上下文请使用 invoke_async".into(),
            ));
        }
        tokio::task::block_in_place(|| executor()?.runtime.block_on(future))
    } else {
        executor()?.runtime.block_on(future)
    }
}
/// Stop/join both executors before releasing their module references. Stop host
/// entry calls and drain task services first. In-flight work or external executor
/// references return an error without removing either pool. This is a Rust API;
/// no C export or existing invocation contract is changed.
pub fn shutdown_executor() -> MiresidianResult<()> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(MiresidianError::NotSupported(
            "执行器须在同步上下文关闭".into(),
        ));
    }
    let (business, lifecycle) = {
        let mut pools = EXECUTORS
            .lock()
            .map_err(|_| MiresidianError::Internal("执行器锁污染".into()))?;
        if pools.stopping
            || ACTIVE.load(Ordering::Acquire) > 0
            || pools
                .business
                .as_ref()
                .is_some_and(|p| Arc::strong_count(p) != 1)
            || pools
                .lifecycle
                .as_ref()
                .is_some_and(|p| Arc::strong_count(p) != 1)
        {
            return Err(MiresidianError::Failure("执行器仍有在途调用或任务".into()));
        }
        pools.stopping = true;
        (pools.business.take(), pools.lifecycle.take())
    };
    // Do not hold registry/loader locks while waiting for worker destruction.
    drop(business);
    drop(lifecycle);
    EXECUTORS
        .lock()
        .map_err(|_| MiresidianError::Internal("执行器锁污染".into()))?
        .stopping = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_preserves_both_pools_when_lifecycle_is_busy_or_borrowed() {
        let business = executor().unwrap();
        let weak_business = Arc::downgrade(&business);
        drop(business);
        let lifecycle = lifecycle_executor().unwrap();
        let weak_lifecycle = Arc::downgrade(&lifecycle);
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let work = lifecycle.spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        });
        drop(lifecycle);
        started_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(shutdown_executor().is_err());
        assert!(weak_business.upgrade().is_some());
        assert!(weak_lifecycle.upgrade().is_some());
        release_tx.send(()).unwrap();
        block_on(async {
            work.await
                .map_err(|e| MiresidianError::Internal(e.to_string()))
        })
        .unwrap();
        let borrowed = lifecycle_executor().unwrap();
        assert!(shutdown_executor().is_err());
        assert!(weak_business.upgrade().is_some());
        drop(borrowed);
        shutdown_executor().unwrap();
        assert!(weak_business.upgrade().is_none());
        assert!(weak_lifecycle.upgrade().is_none());
    }
}
