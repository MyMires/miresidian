//! 有界任务所有权与跨线程结果读取；任务和结果持续占容量直到显式释放。
mod entry;
use crate::{
    CallContext, Invocation, MiresidianError, MiresidianResult, Node, NodeRuntime, Platform,
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
/// 持有运行时句柄并管理后台请求、结果和任务 ID 的服务。
///
/// 完成结果在显式 release 前仍占容量；取消仅发出协作信号，不强制终止业务。
/// shutdown 永久停止本服务提交并等待任务、清理运行时；其他 Clone 共用该运行时。
/// DLL 卸载前还须停止宿主并发入口并回收框架执行器，不能只丢弃服务或等待 Future。
pub struct TaskService<N: Node> {
    runtime: NodeRuntime<N>,
    table: Mutex<Table>,
    next: AtomicU64,
    capacity: usize,
    drain: tokio::sync::Mutex<()>,
}
impl<N: Node> TaskService<N> {
    /// 用给定运行时创建任务表；capacity 须在 1..=65536，否则返回 InvalidArgument。
    /// 容量同时计算运行中任务与未释放结果；创建服务不会初始化节点。
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
    /// 提交请求并返回本服务内不复用的任务 ID；容量满或服务已关闭时返回错误。
    ///
    /// platform=None 使用默认平台路径，Some 使用显式平台路径及其 Config 布局。
    /// timeout 从上下文创建时计算并包含排队，None 不设置截止时间；提交成功不代表业务成功。
    pub fn submit(
        &self,
        request: Invocation,
        platform: Option<Platform>,
        timeout: Option<Duration>,
    ) -> MiresidianResult<u64> {
        self.submit_context(request, platform, CallContext::new(timeout))
    }
    /// 用调用方上下文提交请求；返回任务 ID，容量和服务关闭错误与 submit 相同。
    /// 上下文可预先取消或已到期；其截止时间不从本次提交重新计算。
    pub fn submit_context(
        &self,
        request: Invocation,
        platform: Option<Platform>,
        context: CallContext,
    ) -> MiresidianResult<u64> {
        self.submit_work(Some((request, platform)), None, context)
    }
    /// 提交各自携带目标 Config 的批次，返回代表整个批次的任务 ID。
    ///
    /// 超过运行时批次限额、服务容量已满或已关闭时返回错误。任务输出按原请求顺序
    /// 保存每项 JSON 响应，各自含 status/code/message/output；单项业务错误不取消其他项。
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
    /// 立即复制任务快照，不等待业务，不消费结果；未知或已释放 ID 返回 InvalidArgument。
    pub fn poll(&self, id: u64) -> MiresidianResult<TaskSnapshot> {
        Ok(self.entry(id)?.snapshot())
    }
    /// 异步等待任务完成或本次等待超时，返回当时快照；未知或已释放 ID 返回参数错误。
    ///
    /// timeout=None 等待至完成；等待超时可能返回非终态，不取消任务、不释放结果，
    /// 也不修改提交时的截止时间。可从其他线程继续查询或等待同一 ID。
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
    /// wait_async 的同步桥；等待预算不改变业务截止时间，单线程异步上下文中返回能力错误。
    pub fn wait(&self, id: u64, timeout: Option<Duration>) -> MiresidianResult<TaskSnapshot> {
        super::execution::block_on(self.wait_async(id, timeout))
    }
    /// 为未完成任务发出取消信号并返回即时快照，不等待停止或回滚设备动作。
    /// 已完成任务保留结果；未知或已释放 ID 返回 InvalidArgument。
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
    /// 移除已实际结束任务及其结果，归还服务容量；未知 ID 或任务仍在运行时返回错误。
    /// cancel 信号或终态快照不代替后台任务结束，不能据此提前释放运行资源。
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
    /// 永久停止本服务的新提交，发出协作取消并等待全部任务，再关闭其共用运行时。
    ///
    /// 业务若忽略取消，会继续等待其自行返回或设备超时；不强杀线程。
    /// 已有结果保留供查询和释放；清理错误返回调用方。同步入口不支持单线程异步上下文。
    pub fn shutdown(&self) -> MiresidianResult<()> {
        super::execution::block_on(self.shutdown_async())
    }

    /// shutdown 的异步形式，可在单线程调用方执行器中等待；相同停止、等待和清理语义。
    /// 丢弃等待 Future 后服务仍保持停止提交，下次关闭可继续排空任务。
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
        // 全部任务结束前保留登记表中的 JoinHandle；本次等待被丢弃后，下次关闭仍可排空，
        // 避免只取走句柄就使在途任务脱离服务的关闭流程。
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
