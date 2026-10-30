//! Isolated integration process: joining the global executor cannot disrupt
//! other integration tests' runtimes.
use miresidian::{Invocation, MiresidianResult, NodeRuntime, TaskService, shutdown_executor};
use std::{
    sync::{Condvar, Mutex, OnceLock, mpsc},
    time::Duration,
};
static STARTED: OnceLock<mpsc::Sender<()>> = OnceLock::new();
static RELEASE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());
#[miresidian::node("执行器退出验证")]
struct ShutdownNode;
impl ShutdownNode {
    #[miresidian::operation]
    fn init_device() -> MiresidianResult<Self> {
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation]
    fn hold(&mut self) -> MiresidianResult<()> {
        STARTED.get().unwrap().send(()).unwrap();
        let lock = RELEASE.0.lock().unwrap();
        let (_guard, t) = RELEASE
            .1
            .wait_timeout_while(lock, Duration::from_secs(5), |s| !*s)
            .unwrap();
        assert!(!t.timed_out());
        Ok(())
    }
}
#[test]
fn executor_rejects_early_shutdown_then_drains_and_joins() {
    let (tx, rx) = mpsc::channel();
    STARTED.set(tx).unwrap();
    let runtime = NodeRuntime::<ShutdownNode>::new();
    runtime
        .invoke(&Invocation::new("InitDevice", vec![], vec![]))
        .unwrap();
    let service = TaskService::new(runtime, 1).unwrap();
    let id = service
        .submit(Invocation::new("Hold", vec![], vec![]), None, None)
        .unwrap();
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(shutdown_executor().is_err());
    *RELEASE.0.lock().unwrap() = true;
    RELEASE.1.notify_all();
    service.wait(id, None).unwrap();
    service.shutdown().unwrap();
    service.release(id).unwrap();
    shutdown_executor().unwrap();
}
