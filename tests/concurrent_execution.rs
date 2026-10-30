#![cfg(feature = "instances")]
//! Execution overlap and lifecycle tests use explicit start/release signals.
use miresidian::{
    CallContext, ExecutionLimits, Invocation, MiresidianResult, NodeRuntime, PlatformRegistry,
    TaskService, TaskState,
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Probe {
    state: Mutex<(usize, bool, Vec<String>)>,
    changed: Condvar,
    async_release: tokio::sync::Notify,
    closed: AtomicUsize,
}
impl Probe {
    fn start(&self, label: &str) {
        let mut state = self.state.lock().unwrap();
        state.0 += 1;
        state.2.push(label.into());
        self.changed.notify_all();
    }
    fn wait(&self, count: usize) {
        let state = self.state.lock().unwrap();
        let (state, t) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(5), |s| s.0 < count)
            .unwrap();
        assert!(!t.timed_out(), "execution never overlapped: {:?}", state.2);
    }
    fn release(&self) {
        self.state.lock().unwrap().1 = true;
        self.changed.notify_all();
        self.async_release.notify_one();
    }
    fn hold(&self) {
        let state = self.state.lock().unwrap();
        let (_state, t) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(5), |s| !s.1)
            .unwrap();
        assert!(!t.timed_out(), "test did not release business");
    }
}
fn probes() -> &'static Mutex<HashMap<String, Arc<Probe>>> {
    static VALUE: OnceLock<Mutex<HashMap<String, Arc<Probe>>>> = OnceLock::new();
    VALUE.get_or_init(Mutex::default)
}
fn probe(key: &str) -> Arc<Probe> {
    probes()
        .lock()
        .unwrap()
        .entry(key.into())
        .or_default()
        .clone()
}

#[miresidian::node("并发验证",instances(mode=multiple,binding=config,execution=selectable,default_execution=sequential))]
struct ConcurrentNode {
    probe: Arc<Probe>,
    calls: usize,
}
impl ConcurrentNode {
    #[miresidian::operation]
    fn init_device(address: String) -> MiresidianResult<Self> {
        let probe = probe(&address);
        if address.starts_with("init-hold") {
            probe.start("init");
            probe.hold();
        }
        Ok(Self { probe, calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        self.probe.closed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    #[miresidian::operation]
    fn record(&mut self, label: String) -> MiresidianResult<String> {
        self.calls += 1;
        self.probe.start(&label);
        if label == "hold" {
            self.probe.hold();
        }
        Ok(format!("{label}:{}", self.calls))
    }
    #[miresidian::operation]
    async fn suspend(&mut self) -> MiresidianResult<String> {
        self.probe.start("async");
        self.probe.async_release.notified().await;
        CallContext::current().check()?;
        Ok("async complete".into())
    }
    #[miresidian::operation]
    async fn cancel_wait(&mut self) -> MiresidianResult<()> {
        self.probe.start("cancel_wait");
        let context = CallContext::current();
        context.cancelled().await;
        context.check()
    }
    #[miresidian::operation]
    async fn explode(&mut self) -> MiresidianResult<()> {
        tokio::task::yield_now().await;
        panic!("async panic")
    }
}
fn request(op: &str, key: &str, input: &[&str]) -> Invocation {
    Invocation::new(
        op,
        vec![key.into()],
        input.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
    )
}
fn init(runtime: &NodeRuntime<ConcurrentNode>, key: &str, mode: &str) {
    runtime
        .invoke(&Invocation::new(
            "InitDevice",
            vec![key.into(), mode.into()],
            vec![],
        ))
        .unwrap();
}
fn start(
    runtime: NodeRuntime<ConcurrentNode>,
    key: &str,
    op: &str,
    input: &[&str],
) -> std::thread::JoinHandle<MiresidianResult<miresidian::InvocationOutput>> {
    let req = request(op, key, input);
    std::thread::spawn(move || runtime.invoke(&req))
}

#[test]
fn independent_instances_execute_while_another_is_blocked() {
    let runtime = NodeRuntime::new();
    init(&runtime, "overlap-a", "Sequential");
    init(&runtime, "overlap-b", "Sequential");
    let a = probe("overlap-a");
    let b = probe("overlap-b");
    let first = start(runtime.clone(), "overlap-a", "Record", &["hold"]);
    a.wait(1);
    let second = start(runtime.clone(), "overlap-b", "Record", &["hold"]);
    b.wait(1);
    assert_eq!(
        runtime
            .invoke(&request("GetFrameworkVersion", "", &[]))
            .unwrap()
            .as_slice(),
        [miresidian::MIRESIDIAN_VERSION]
    );
    a.release();
    b.release();
    first.join().unwrap().unwrap();
    second.join().unwrap().unwrap();
}
#[test]
fn shared_batch_excludes_other_requests_in_every_mode() {
    for mode in ["Sequential", "Parallel", "Broadcast"] {
        let key = format!("batch-{mode}");
        let runtime = NodeRuntime::new();
        init(&runtime, &key, mode);
        let p = probe(&key);
        let batch = start(
            runtime.clone(),
            &key,
            "Record",
            &["共享", "hold", "hold", "hold"],
        );
        p.wait(1);
        let other = start(runtime.clone(), &key, "Record", &["other"]);
        p.release();
        let batch = batch.join().unwrap().unwrap();
        other.join().unwrap().unwrap();
        assert_eq!(batch.as_slice().len(), 3);
        if mode == "Broadcast" {
            assert_eq!(p.state.lock().unwrap().2, ["hold", "other"]);
        } else {
            assert_eq!(p.state.lock().unwrap().2, ["hold", "hold", "hold", "other"]);
        }
    }
}
#[test]
fn closing_one_instance_waits_without_blocking_another() {
    let runtime = NodeRuntime::new();
    init(&runtime, "close-a", "Sequential");
    init(&runtime, "close-b", "Sequential");
    let p = probe("close-a");
    let work = start(runtime.clone(), "close-a", "Record", &["hold"]);
    p.wait(1);
    let close = start(runtime.clone(), "close-a", "CloseDevice", &[]);
    assert_eq!(
        runtime
            .invoke(&request("Record", "close-b", &["free"]))
            .unwrap()
            .as_slice(),
        ["free:1"]
    );
    assert_eq!(p.closed.load(Ordering::SeqCst), 0);
    p.release();
    work.join().unwrap().unwrap();
    close.join().unwrap().unwrap();
    assert_eq!(p.closed.load(Ordering::SeqCst), 1);
}
#[test]
fn shutdown_cleans_an_initialization_that_was_already_running() {
    let runtime = NodeRuntime::<ConcurrentNode>::new();
    let p = probe("init-hold-shutdown");
    let init = start(runtime.clone(), "init-hold-shutdown", "InitDevice", &[]);
    p.wait(1);
    let copy = runtime.clone();
    let close = std::thread::spawn(move || copy.shutdown());
    // Wait for the observable admission barrier before releasing construction.
    for _ in 0..10000 {
        if runtime
            .invoke(&request("Record", "absent", &["x"]))
            .unwrap_err()
            .to_string()
            .contains("正在关闭")
        {
            break;
        }
        std::thread::yield_now();
    }
    p.release();
    let _ = init.join().unwrap();
    close.join().unwrap().unwrap();
    assert!(!runtime.is_initialized());
    assert_eq!(p.closed.load(Ordering::SeqCst), 1);
}
#[test]
fn native_async_operations_release_executor_threads_and_isolate_panics() {
    let runtime = NodeRuntime::with_limits(
        PlatformRegistry::default(),
        ExecutionLimits {
            max_running: 64,
            ..Default::default()
        },
    )
    .unwrap();
    let service = TaskService::new(runtime.clone(), 64).unwrap();
    let mut ids = Vec::new();
    // More suspended futures than the blocking pool capacity. A sync wrapper
    // masquerading as async would exhaust that pool and fail this start barrier.
    for n in 0..40 {
        let key = format!("native-{n}");
        init(&runtime, &key, "Sequential");
        ids.push((
            service
                .submit(request("Suspend", &key, &[]), None, None)
                .unwrap(),
            key,
        ));
    }
    for (_, key) in &ids {
        probe(key).wait(1);
    }
    for (_, key) in &ids {
        probe(key).release();
    }
    for (id, _) in ids {
        assert_eq!(
            service
                .wait(id, Some(Duration::from_secs(5)))
                .unwrap()
                .state,
            TaskState::Succeeded
        );
    }
    init(&runtime, "native-panic", "Sequential");
    let id = service
        .submit(request("Explode", "native-panic", &[]), None, None)
        .unwrap();
    assert_eq!(service.wait(id, None).unwrap().code, "NODE_PANICKED");
    assert!(matches!(
        runtime.invoke(&request("Record", "native-panic", &["x"])),
        Err(miresidian::MiresidianError::NodePanicked { .. })
    ));
    service.shutdown().unwrap();
}
#[test]
fn queued_cancellation_and_deadline_prevent_business_execution() {
    let runtime = NodeRuntime::new();
    init(&runtime, "task-queue", "Sequential");
    let p = probe("task-queue");
    let service = TaskService::new(runtime.clone(), 3).unwrap();
    let first = service
        .submit(request("Record", "task-queue", &["hold"]), None, None)
        .unwrap();
    p.wait(1);
    let cancelled = service
        .submit(request("Record", "task-queue", &["cancelled"]), None, None)
        .unwrap();
    service.cancel(cancelled).unwrap();
    let expired = service
        .submit(
            request("Record", "task-queue", &["expired"]),
            None,
            Some(Duration::from_millis(10)),
        )
        .unwrap();
    assert!(
        service
            .submit(request("Record", "task-queue", &["overflow"]), None, None)
            .is_err()
    );
    assert_eq!(
        service
            .wait(cancelled, Some(Duration::from_secs(2)))
            .unwrap()
            .state,
        TaskState::Cancelled
    );
    assert_eq!(
        service
            .wait(expired, Some(Duration::from_secs(2)))
            .unwrap()
            .state,
        TaskState::TimedOut
    );
    assert!(service.release(first).is_err());
    p.release();
    assert_eq!(
        service.wait(first, None).unwrap().state,
        TaskState::Succeeded
    );
    assert_eq!(p.state.lock().unwrap().2, ["hold"]);
    service.shutdown().unwrap();
}
#[test]
fn cooperative_native_cancellation_retains_ownership_until_return() {
    let runtime = NodeRuntime::new();
    init(&runtime, "cooperative", "Sequential");
    let service = TaskService::new(runtime.clone(), 2).unwrap();
    let id = service
        .submit(request("CancelWait", "cooperative", &[]), None, None)
        .unwrap();
    probe("cooperative").wait(1);
    service.cancel(id).unwrap();
    assert_eq!(
        service
            .wait(id, Some(Duration::from_secs(2)))
            .unwrap()
            .state,
        TaskState::Cancelled
    );
    service.shutdown().unwrap();
}
#[test]
fn bounded_instance_queue_does_not_consume_all_running_slots() {
    let limits = ExecutionLimits {
        max_running: 1,
        max_pending_per_instance: 1,
        ..Default::default()
    };
    let runtime = NodeRuntime::with_limits(PlatformRegistry::default(), limits).unwrap();
    init(&runtime, "limit-a", "Sequential");
    init(&runtime, "limit-b", "Sequential");
    let p = probe("limit-a");
    let work = start(runtime.clone(), "limit-a", "Record", &["hold"]);
    p.wait(1);
    assert!(
        runtime
            .invoke(&request("Record", "limit-a", &["overflow"]))
            .unwrap_err()
            .to_string()
            .contains("队列已满")
    );
    p.release();
    work.join().unwrap().unwrap();
    assert!(
        runtime
            .invoke(&request("Record", "limit-b", &["free"]))
            .is_ok()
    );
}

#[test]
fn both_exported_sync_entries_share_identity_without_serializing_business() {
    fn ffi(op: &str, key: &str, input: &[&str], explicit: bool) -> serde_json::Value {
        use std::ffi::{CStr, CString};
        let operation = CString::new(op).unwrap();
        let config = CString::new(serde_json::to_string(&[key]).unwrap()).unwrap();
        let input = CString::new(serde_json::to_string(input).unwrap()).unwrap();
        // SAFETY: all input buffers are live C strings; response is copied before another call.
        let response = unsafe {
            let p = if explicit {
                miresidian_invoke_with(1, operation.as_ptr(), config.as_ptr(), input.as_ptr())
            } else {
                miresidian_invoke(operation.as_ptr(), config.as_ptr(), input.as_ptr())
            };
            CStr::from_ptr(p).to_str().unwrap().to_owned()
        };
        serde_json::from_str(&response).unwrap()
    }
    assert_eq!(ffi("InitDevice", "ffi-overlap-a", &[], false)["code"], "OK");
    assert_eq!(ffi("InitDevice", "ffi-overlap-b", &[], true)["code"], "OK");
    let a = probe("ffi-overlap-a");
    let b = probe("ffi-overlap-b");
    let first = std::thread::spawn(|| ffi("Record", "ffi-overlap-a", &["hold"], false));
    a.wait(1);
    let second = std::thread::spawn(|| ffi("Record", "ffi-overlap-b", &["hold"], true));
    b.wait(1);
    a.release();
    b.release();
    assert_eq!(first.join().unwrap()["code"], "OK");
    assert_eq!(second.join().unwrap()["code"], "OK");
    ffi("CloseDevice", "ffi-overlap-a", &[], false);
    ffi("CloseDevice", "ffi-overlap-b", &[], true);
}

#[test]
fn explicit_cross_instance_batch_preserves_results_and_settles_all_items() {
    let runtime = NodeRuntime::new();
    init(&runtime, "explicit-a", "Sequential");
    init(&runtime, "explicit-b", "Sequential");
    let a = probe("explicit-a");
    let b = probe("explicit-b");
    let copy = runtime.clone();
    let batch = std::thread::spawn(move || {
        copy.invoke_batch(vec![
            request("Record", "explicit-a", &["hold"]),
            request("Record", "missing", &["error"]),
            request("Record", "explicit-b", &["hold"]),
        ])
    });
    a.wait(1);
    b.wait(1);
    a.release();
    b.release();
    let output = batch.join().unwrap().unwrap();
    assert_eq!(output.len(), 3);
    assert!(output[0].is_ok());
    assert!(output[1].is_err());
    assert!(output[2].is_ok());
}

#[test]
fn task_results_survive_cross_thread_polling_and_can_be_released() {
    let runtime = NodeRuntime::new();
    init(&runtime, "result-owner", "Sequential");
    let service = Arc::new(TaskService::new(runtime, 1).unwrap());
    let id = service
        .submit(request("Record", "result-owner", &["value"]), None, None)
        .unwrap();
    let copy = service.clone();
    let snapshot = std::thread::spawn(move || copy.wait(id, None))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.output, ["value:1"]);
    for _ in 0..10000 {
        if service.release(id).is_ok() {
            break;
        }
        std::thread::yield_now();
    }
    assert!(service.poll(id).is_err());
    service.shutdown().unwrap();
    assert!(
        service
            .submit(request("GetFunctions", "", &[]), None, None)
            .is_err()
    );
}

#[test]
fn dropping_the_callers_executor_does_not_abort_an_admitted_request() {
    let runtime = NodeRuntime::new();
    init(&runtime, "caller-drop", "Sequential");
    let p = probe("caller-drop");
    let caller = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let copy = runtime.clone();
    caller.block_on(async {
        tokio::spawn(async move {
            copy.invoke_async(request("Suspend", "caller-drop", &[]))
                .await
        });
        tokio::task::yield_now().await;
        p.wait(1);
    });
    drop(caller);
    let close = start(runtime.clone(), "caller-drop", "CloseDevice", &[]);
    assert_eq!(p.closed.load(Ordering::SeqCst), 0);
    p.release();
    close.join().unwrap().unwrap();
    assert_eq!(p.closed.load(Ordering::SeqCst), 1);
}

#[test]
fn concurrent_same_key_initialization_constructs_only_once() {
    let runtime = NodeRuntime::<ConcurrentNode>::new();
    let p = probe("init-hold-once");
    let first = start(runtime.clone(), "init-hold-once", "InitDevice", &[]);
    p.wait(1);
    let second = start(runtime.clone(), "init-hold-once", "InitDevice", &[]);
    p.release();
    first.join().unwrap().unwrap();
    let reused = second.join().unwrap().unwrap();
    assert_eq!(reused.as_slice(), [miresidian::CHANNEL_REUSED_MESSAGE]);
    assert_eq!(p.state.lock().unwrap().0, 1);
}

#[test]
fn submitted_batches_keep_per_item_errors_and_result_positions() {
    let runtime = NodeRuntime::new();
    init(&runtime, "submitted-batch", "Sequential");
    let service = TaskService::new(runtime, 1).unwrap();
    let id = service
        .submit_batch(
            vec![
                request("Record", "submitted-batch", &["first"]),
                request("Record", "unknown", &["bad"]),
            ],
            None,
        )
        .unwrap();
    let output = service.wait(id, None).unwrap();
    assert_eq!(output.state, TaskState::Succeeded);
    assert_eq!(output.output.len(), 2);
    let first: serde_json::Value = serde_json::from_str(&output.output[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(&output.output[1]).unwrap();
    assert_eq!(first["code"], "OK");
    assert_eq!(second["code"], "UNKNOWN_CHANNEL");
    service.shutdown().unwrap();
}
