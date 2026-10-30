//! Cancellation never destroys a running business future or releases its instance.
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

/// Cooperative cancellation and absolute deadline, measured from submission.
/// Running synchronous code must check this context or use its own I/O timeout.
/// A cancellation request is distinct from confirmed task completion.
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
    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }
    pub fn cancel(&self) {
        self.cancel.send_replace(true);
    }
    pub fn is_cancelled(&self) -> bool {
        *self.cancel.borrow()
    }
    pub fn is_expired(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }
    pub fn remaining(&self) -> Option<Duration> {
        self.deadline
            .map(|d| d.saturating_duration_since(Instant::now()))
    }
    /// Available inside both macro-generated sync and async business methods.
    pub fn current() -> Self {
        ASYNC_CONTEXT
            .try_with(Clone::clone)
            .ok()
            .or_else(|| SYNC_CONTEXT.with(|c| c.borrow().clone()))
            .unwrap_or_default()
    }
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
