//! 业务池与生命周期池持有承载模块的加载引用，回收工作线程后才释放该引用。
//! 先取得模块引用再创建线程；执行器构建失败时也由所有权回收已取得的引用。
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
    // 字段按声明顺序析构：先回收 Runtime 工作线程，再释放模块加载引用，不能交换顺序。
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

/// 同步等待框架 Future，并把执行错误原样返回。
///
/// 多线程 Tokio 上下文通过 block_in_place 桥接；单线程 Tokio 上下文返回 NotSupported，
/// 调用方须改用异步 API，避免阻塞唯一负责推进 Future 的线程。
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
/// 停止并回收业务池和生命周期池线程，最后释放两池持有的模块加载引用。
///
/// 调用前须停止宿主入口并排空任务服务。存在在途工作或外部执行器引用时返回错误，
/// 两池均保持原状；异步上下文中返回 NotSupported。成功后允许后续调用重新创建池。
/// 其他模块引用和租约服务仍可延后卸载；此 Rust API 没有对应的节点 C 导出。
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
    // 移出两池后释放登记锁，再等待线程析构；关闭期间 stopping 阻止其他调用重建池。
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
