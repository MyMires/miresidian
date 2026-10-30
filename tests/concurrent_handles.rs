#![cfg(feature = "instances")]
//! Distinct handles preserve independent identity even with identical Config.
use miresidian::{Invocation, MiresidianResult, NodeRuntime};
use std::{
    sync::{Condvar, Mutex, OnceLock, mpsc},
    time::Duration,
};
static STARTED: OnceLock<mpsc::Sender<()>> = OnceLock::new();
static RELEASE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());
#[miresidian::node("Handle 并行验证",instances(mode=multiple,binding=handle,execution=sequential))]
struct HandleNode;
impl HandleNode {
    #[miresidian::operation]
    fn init_device(_address: String) -> MiresidianResult<Self> {
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
        let (_guard, timeout) = RELEASE
            .1
            .wait_timeout_while(lock, Duration::from_secs(5), |value| !*value)
            .unwrap();
        assert!(!timeout.timed_out(), "independent handle failed to overlap");
        Ok(())
    }
}
#[test]
fn distinct_handles_with_identical_business_config_execute_in_parallel() {
    let (tx, rx) = mpsc::channel();
    STARTED.set(tx).unwrap();
    let runtime = NodeRuntime::<HandleNode>::new();
    for handle in ["h1", "h2"] {
        runtime
            .invoke(&Invocation::new(
                "InitDevice",
                vec!["same".into(), handle.into()],
                vec![],
            ))
            .unwrap();
    }
    let mut workers = Vec::new();
    for handle in ["h1", "h2"] {
        let copy = runtime.clone();
        workers.push(std::thread::spawn(move || {
            copy.invoke(&Invocation::new(
                "Hold",
                vec!["same".into(), handle.into()],
                vec![],
            ))
        }));
    }
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    *RELEASE.0.lock().unwrap() = true;
    RELEASE.1.notify_all();
    for worker in workers {
        worker.join().unwrap().unwrap();
    }
}
