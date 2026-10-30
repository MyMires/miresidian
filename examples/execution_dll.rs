//! In-workspace, hardware-free ABI execution example. This is part of the
//! framework crate, not an independent project with a local framework override.
use miresidian::{CallContext, MiresidianError, MiresidianResult};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
static SYNC_JOINED: AtomicUsize = AtomicUsize::new(0);
static ASYNC_JOINED: AtomicUsize = AtomicUsize::new(0);
#[miresidian::node("执行器验证",instances(mode=multiple,binding=config,execution=selectable,default_execution=sequential))]
struct ExecutionNode {
    calls: u32,
}
impl ExecutionNode {
    #[miresidian::operation]
    fn init_device(_address: String) -> MiresidianResult<Self> {
        Ok(Self { calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation]
    fn pair(&mut self) -> MiresidianResult<u32> {
        SYNC_JOINED.fetch_add(1, Ordering::AcqRel);
        let deadline = Instant::now() + Duration::from_secs(3);
        while SYNC_JOINED.load(Ordering::Acquire) < 2 {
            if Instant::now() > deadline {
                return Err(MiresidianError::Failure("同步实例未重叠".into()));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        self.calls += 1;
        Ok(self.calls)
    }
    #[miresidian::operation]
    async fn pair_async(&mut self) -> MiresidianResult<u32> {
        ASYNC_JOINED.fetch_add(1, Ordering::AcqRel);
        let deadline = Instant::now() + Duration::from_secs(3);
        while ASYNC_JOINED.load(Ordering::Acquire) < 2 {
            if Instant::now() > deadline {
                return Err(MiresidianError::Failure("异步实例未重叠".into()));
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        self.calls += 1;
        Ok(self.calls)
    }
    #[miresidian::operation]
    fn delay(&mut self, milliseconds: u64) -> MiresidianResult<()> {
        std::thread::sleep(Duration::from_millis(milliseconds));
        Ok(())
    }
    #[miresidian::operation]
    async fn delay_async(&mut self, milliseconds: u64) -> MiresidianResult<()> {
        tokio::time::sleep(Duration::from_millis(milliseconds)).await;
        Ok(())
    }
    #[miresidian::operation]
    async fn wait_cancel(&mut self) -> MiresidianResult<()> {
        let context = CallContext::current();
        context.cancelled().await;
        context.check()
    }
}
