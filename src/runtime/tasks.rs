//! Bounded task ownership and cross-thread result retrieval.
mod entry;
use crate::{
    CallContext, DeviceNode, Invocation, MiresidianError, MiresidianResult, NodeRuntime, Platform,
};
use entry::TaskEntry;
pub use entry::{TaskSnapshot, TaskState};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

struct Table {
    closed: bool,
    drained: bool,
    entries: BTreeMap<u64, Arc<TaskEntry>>,
}
/// Runtime-owned task service. Completed results count against capacity until
/// explicitly released. Cancellation requests never forcibly terminate user code.
/// Call shutdown and join the framework executor before unloading a DLL.
pub struct TaskService<N: DeviceNode> {
    runtime: NodeRuntime<N>,
    table: Mutex<Table>,
    next: AtomicU64,
    capacity: usize,
    drain: tokio::sync::Mutex<()>,
}
impl<N: DeviceNode> TaskService<N> {
    pub fn new(runtime: NodeRuntime<N>, capacity: usize) -> MiresidianResult<Self> {
        if capacity == 0 || capacity > 65536 {
            return Err(MiresidianError::InvalidArgument {
                field: "capacity".into(),
                message: "须在 1..=65536".into(),
            });
        }
        Ok(Self {
            runtime,
            table: Mutex::new(Table {
                closed: false,
                drained: false,
                entries: BTreeMap::new(),
            }),
            next: AtomicU64::new(1),
            capacity,
            drain: tokio::sync::Mutex::new(()),
        })
    }
    /// Submit a request. None selects the default platform path; Some uses the
    /// explicit-platform path with the same Config rules as invoke_with.
    pub fn submit(
        &self,
        request: Invocation,
        platform: Option<Platform>,
        timeout: Option<Duration>,
    ) -> MiresidianResult<u64> {
        self.submit_context(request, platform, CallContext::new(timeout))
    }
    pub fn submit_context(
        &self,
        request: Invocation,
        platform: Option<Platform>,
        context: CallContext,
    ) -> MiresidianResult<u64> {
        self.submit_work(Some((request, platform)), None, context)
    }
    /// Submit an explicitly targeted batch. Each output is the corresponding
    /// request's JSON result, including its own status/code/message/output.
    pub fn submit_batch(
        &self,
        requests: Vec<Invocation>,
        timeout: Option<Duration>,
    ) -> MiresidianResult<u64> {
        if requests.len() > self.runtime.batch_limit() {
            return Err(MiresidianError::Failure("批次超过容量限制".into()));
        }
        self.submit_work(None, Some(requests), CallContext::new(timeout))
    }
    fn submit_work(
        &self,
        single: Option<(Invocation, Option<Platform>)>,
        batch: Option<Vec<Invocation>>,
        context: CallContext,
    ) -> MiresidianResult<u64> {
        let mut table = self.table.lock().unwrap_or_else(|e| e.into_inner());
        if table.closed {
            return Err(MiresidianError::Failure("任务服务已停止".into()));
        }
        if table.entries.len() >= self.capacity {
            return Err(MiresidianError::Failure(
                "任务容量已满；请释放已完成结果".into(),
            ));
        }
        let id = self
            .next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| MiresidianError::Failure("任务 ID 已耗尽".into()))?;
        let entry = Arc::new(TaskEntry::new(id, context.clone()));
        let state = entry.clone();
        let runtime = self.runtime.clone();
        let executor = super::execution::executor()?;
        let activity = super::execution::Activity::new();
        let handle = executor.spawn(async move {
            let _activity = activity;
            use futures_util::FutureExt;
            let call = async {
                if let Some((request, platform)) = single {
                    match platform {
                        Some(p) => {
                            runtime
                                .invoke_platform_async(p, request, context.clone())
                                .await
                        }
                        None => runtime.invoke_with_context(request, context.clone()).await,
                    }
                } else {
                    context.check()?;
                    let results = futures_util::future::join_all(
                        batch
                            .unwrap_or_default()
                            .into_iter()
                            .map(|request| runtime.invoke_with_context(request, context.clone())),
                    )
                    .await;
                    let output = results
                        .into_iter()
                        .map(entry::batch_result)
                        .collect::<MiresidianResult<Vec<_>>>()?;
                    Ok(crate::InvocationOutput::new(output))
                }
            };
            let result = std::panic::AssertUnwindSafe(call)
                .catch_unwind()
                .await
                .unwrap_or_else(|_| Err(MiresidianError::Internal("任务异常".into())));
            *state.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
            state.changed.notify_waiters();
            drop(runtime);
        });
        *entry.handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
        table.entries.insert(id, entry);
        Ok(id)
    }
    fn entry(&self, id: u64) -> MiresidianResult<Arc<TaskEntry>> {
        self.table
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .get(&id)
            .cloned()
            .ok_or_else(|| MiresidianError::InvalidArgument {
                field: "task_id".into(),
                message: "任务不存在或已释放".into(),
            })
    }
    pub fn poll(&self, id: u64) -> MiresidianResult<TaskSnapshot> {
        Ok(self.entry(id)?.snapshot())
    }
    /// Wait at most timeout for completion; expiration returns a nonterminal
    /// snapshot and does not cancel or release the task.
    pub async fn wait_async(
        &self,
        id: u64,
        timeout: Option<Duration>,
    ) -> MiresidianResult<TaskSnapshot> {
        let entry = self.entry(id)?;
        let wait = async {
            loop {
                let notified = entry.changed.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                if entry
                    .result
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .is_some()
                {
                    break;
                }
                notified.await;
            }
        };
        match timeout {
            Some(t) => {
                let _ = tokio::time::timeout(t, wait).await;
            }
            None => wait.await,
        };
        Ok(entry.snapshot())
    }
    pub fn wait(&self, id: u64, timeout: Option<Duration>) -> MiresidianResult<TaskSnapshot> {
        super::execution::block_on(self.wait_async(id, timeout))
    }
    pub fn cancel(&self, id: u64) -> MiresidianResult<TaskSnapshot> {
        let entry = self.entry(id)?;
        if entry
            .result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none()
        {
            entry.context.cancel();
        }
        Ok(entry.snapshot())
    }
    /// Release only a fully completed task, never an in-flight execution.
    pub fn release(&self, id: u64) -> MiresidianResult<()> {
        let mut table = self.table.lock().unwrap_or_else(|e| e.into_inner());
        let entry = table
            .entries
            .get(&id)
            .ok_or_else(|| MiresidianError::InvalidArgument {
                field: "task_id".into(),
                message: "任务不存在".into(),
            })?;
        if !(table.drained
            && entry
                .result
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_some())
            && !entry
                .handle
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .is_some_and(|h| h.is_finished())
        {
            return Err(MiresidianError::Failure("任务尚未结束，不能释放".into()));
        }
        table.entries.remove(&id);
        Ok(())
    }
    /// Permanently stop submission, request cooperative cancellation and wait
    /// for every owned task before cleaning runtime instances. Blocking business
    /// may extend this wait until its own timeout; no thread is force-killed.
    pub fn shutdown(&self) -> MiresidianResult<()> {
        super::execution::block_on(self.shutdown_async())
    }

    /// Async equivalent of shutdown; safe on a single-thread caller executor.
    pub async fn shutdown_async(&self) -> MiresidianResult<()> {
        let _drain = self.drain.lock().await;
        let entries = {
            let mut table = self.table.lock().unwrap_or_else(|e| e.into_inner());
            table.closed = true;
            table.entries.values().cloned().collect::<Vec<_>>()
        };
        for entry in &entries {
            entry.context.cancel();
        }
        // Keep JoinHandles in their entries until every task is finished. If
        // this waiting future is dropped, the next shutdown can still drain it.
        loop {
            let complete = entries.iter().all(|entry| {
                entry
                    .handle
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .is_none_or(|h| h.is_finished())
            });
            if complete {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let handles = entries
            .iter()
            .filter_map(|entry| {
                entry
                    .handle
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
            })
            .collect::<Vec<_>>();
        let results = futures_util::future::join_all(handles).await;
        let runtime = self.runtime.clone();
        let cleanup = super::execution::lifecycle_executor()?
            .spawn_blocking(move || runtime.shutdown())
            .await
            .map_err(|e| MiresidianError::Internal(e.to_string()))?;
        self.table.lock().unwrap_or_else(|e| e.into_inner()).drained = true;
        for result in results {
            result.map_err(|e| MiresidianError::Internal(e.to_string()))?;
        }
        cleanup
    }
}
