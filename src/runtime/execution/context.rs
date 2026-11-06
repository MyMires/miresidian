//! 协作取消与截止时间；请求取消不会销毁在途业务 Future 或提前释放实例执行权。
use crate::{MiresidianError, MiresidianResult};
use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::watch;

tokio::task_local! { static ASYNC_CONTEXT: CallContext; }
thread_local! { static SYNC_CONTEXT: RefCell<Option<CallContext>> = const { RefCell::new(None) }; }

/// 可克隆的请求上下文，共享取消信号和已开始标志，保存从创建时计算的截止时间。
///
/// 排队时间计入预算；同步业务须主动检查或设置设备 I/O 超时。取消请求不等于任务已完成，
/// 业务忽略信号仍可能成功。克隆不会重置截止时间，也不能撤回已发送的取消信号。
#[derive(Clone, Debug)]
pub struct CallContext {
    cancel: Arc<watch::Sender<bool>>,
    deadline: Option<Instant>,
    started: Arc<AtomicBool>,
}
impl Default for CallContext {
    fn default() -> Self {
        Self::new(None)
    }
}
impl CallContext {
    /// 创建独立上下文；timeout 为从此刻起的总预算，None 表示不设置截止时间。
    /// 超过 Instant 可表示范围的预算按无截止时间处理，不影响独立的取消信号。
    pub fn new(timeout: Option<Duration>) -> Self {
        let (tx, _) = watch::channel(false);
        Self {
            started: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(tx),
            deadline: timeout.and_then(|t| Instant::now().checked_add(t)),
        }
    }
    pub(crate) fn mark_started(&self) {
        self.started.store(true, Ordering::Release);
    }
    /// 查询框架是否已标记开始处理；不表示业务已完成，也不保证设备操作已发出。
    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }
    /// 向全部 Clone 发出不可撤回的协作取消信号；立即返回，不等待业务结束。
    pub fn cancel(&self) {
        self.cancel.send_replace(true);
    }
    /// 查询是否收到取消请求；独立于截止时间和最终任务结果。
    pub fn is_cancelled(&self) -> bool {
        *self.cancel.borrow()
    }
    /// 查询已设置的截止时间是否到达；无截止时间时返回 false。
    pub fn is_expired(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }
    /// 返回剩余预算，到期为零；None 表示没有截止时间，而非已经超时。
    pub fn remaining(&self) -> Option<Duration> {
        self.deadline
            .map(|d| d.saturating_duration_since(Instant::now()))
    }
    /// 获取当前框架调用的上下文，兼容宏生成的同步与异步业务。
    /// 框架调用范围外返回独立的默认上下文：不关联已有任务，也不带截止时间。
    pub fn current() -> Self {
        ASYNC_CONTEXT
            .try_with(Clone::clone)
            .ok()
            .or_else(|| SYNC_CONTEXT.with(|c| c.borrow().clone()))
            .unwrap_or_default()
    }
    /// 检查取消和截止时间；正常返回 Ok，失败分别携带 REQUEST_CANCELLED 或 DEADLINE_EXCEEDED。
    /// 同时满足时优先报告取消；调用者自行决定在哪些安全边界停止业务。
    pub fn check(&self) -> MiresidianResult<()> {
        if self.is_cancelled() {
            Err(MiresidianError::Failure("请求已取消".into()).with_context("REQUEST_CANCELLED"))
        } else if self.is_expired() {
            Err(MiresidianError::Failure("请求截止时间已到".into())
                .with_context("DEADLINE_EXCEEDED"))
        } else {
            Ok(())
        }
    }
    /// 等待取消或截止时间到达；已满足时立即返回，可用 check 区分原因。
    /// 丢弃此等待不会取消业务；有截止时间时需在具备 Tokio 时间驱动的执行器中等待。
    pub async fn cancelled(&self) {
        let mut rx = self.cancel.subscribe();
        if self.is_cancelled() || self.is_expired() {
            return;
        }
        match self.remaining() {
            Some(t) => {
                tokio::select! {_ = rx.changed()=>{}, _ = tokio::time::sleep(t)=>{}}
            }
            None => {
                let _ = rx.changed().await;
            }
        }
    }
    pub(crate) async fn scope<T>(&self, f: impl std::future::Future<Output = T>) -> T {
        ASYNC_CONTEXT.scope(self.clone(), f).await
    }
    pub(crate) fn blocking<T>(&self, f: impl FnOnce() -> T) -> T {
        struct Reset(Option<CallContext>);
        impl Drop for Reset {
            fn drop(&mut self) {
                SYNC_CONTEXT.with(|c| *c.borrow_mut() = self.0.take());
            }
        }
        let _reset = Reset(SYNC_CONTEXT.with(|c| c.replace(Some(self.clone()))));
        f()
    }
    pub(crate) async fn wait<T>(
        &self,
        f: impl std::future::Future<Output = T>,
    ) -> MiresidianResult<T> {
        self.check()?;
        tokio::select! { biased; _=self.cancelled()=>{self.check()?; unreachable!()}, value=f=>Ok(value) }
    }
}
