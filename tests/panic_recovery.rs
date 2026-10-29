//! 通过真实属性宏和 FFI 验证 panic 隔离、清理与恢复。
use miresidian::{Invocation, MiresidianError, MiresidianResult, NodeRuntime, ffi_runtime};
use std::{
    ffi::{CStr, CString},
    sync::{Mutex, OnceLock},
};

static CLEANUP: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[miresidian::node("异常恢复测试")]
struct RecoveryNode {
    name: String,
    calls: usize,
}

impl RecoveryNode {
    #[miresidian::operation]
    fn init_device(#[miresidian::config("地址")] name: String) -> MiresidianResult<Self> {
        let node = Self { name, calls: 0 };
        assert_ne!(node.name, "init-panic", "模拟初始化 panic");
        Ok(node)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        CLEANUP.lock().unwrap().push(format!("close:{}", self.name));
        assert!(!self.name.contains("close-panic"), "模拟关闭 panic");
        if self.name.starts_with("close-error") {
            return Err(MiresidianError::Failure("模拟关闭错误".to_owned()));
        }
        Ok(())
    }
    #[miresidian::operation]
    fn get_port_no(
        #[miresidian::input("模式")] mode: String
    ) -> MiresidianResult<Option<String>> {
        let _probe = Self {
            name: "probe".to_owned(),
            calls: 0,
        };
        assert_ne!(mode, "panic", "模拟探测 panic");
        Ok(None)
    }
    #[miresidian::operation("执行")]
    fn run(&mut self, #[miresidian::input("命令")] command: String) -> MiresidianResult<usize> {
        self.calls += 1;
        assert_ne!(command, "panic", "模拟操作 panic");
        Ok(self.calls)
    }
}

impl Drop for RecoveryNode {
    fn drop(&mut self) {
        CLEANUP.lock().unwrap().push(format!("drop:{}", self.name));
        assert!(!self.name.contains("drop-panic"), "模拟析构 panic");
    }
}

type Runtime = OnceLock<Mutex<NodeRuntime<RecoveryNode>>>;

fn call(
    runtime: &Runtime,
    explicit: bool,
    operation: &str,
    config: &[&str],
    input: &[&str],
) -> serde_json::Value {
    let function = CString::new(operation).unwrap();
    let config = CString::new(serde_json::to_string(config).unwrap()).unwrap();
    let input = CString::new(serde_json::to_string(input).unwrap()).unwrap();
    // SAFETY: 全部参数由存活的 CString 提供，响应在下一次调用前立即复制和解析。
    let response = unsafe {
        let pointer = if explicit {
            ffi_runtime::invoke_with(
                runtime,
                1,
                function.as_ptr(),
                config.as_ptr(),
                input.as_ptr(),
            )
        } else {
            ffi_runtime::invoke(runtime, function.as_ptr(), config.as_ptr(), input.as_ptr())
        };
        CStr::from_ptr(pointer).to_str().unwrap()
    };
    serde_json::from_str(response).unwrap()
}

#[test]
fn ffi_panic_isolates_one_channel_and_allows_close_and_reinitialize() {
    for explicit in [false, true] {
        let runtime = Runtime::new();
        for name in ["broken", "healthy"] {
            assert_eq!(
                call(&runtime, explicit, "InitDevice", &[name], &[])["status"],
                0
            );
        }
        assert_eq!(
            call(&runtime, explicit, "执行", &["broken"], &["panic"])["status"],
            3
        );
        assert_eq!(
            call(&runtime, explicit, "执行", &["broken"], &["ok"])["status"],
            3
        );
        assert_eq!(
            call(&runtime, explicit, "InitDevice", &["broken"], &[])["status"],
            3
        );
        assert_eq!(
            call(&runtime, explicit, "执行", &["healthy"], &["ok"])["output"],
            serde_json::json!(["1"])
        );
        for query in [
            "GetFunctions",
            "GetFrameworkVersion",
            "GetProjectVersion",
            "GetInitConfig",
        ] {
            assert_eq!(call(&runtime, explicit, query, &[], &[])["status"], 0);
        }
        assert!(!runtime.get().unwrap().is_poisoned());
        assert_eq!(
            call(&runtime, explicit, "CloseDevice", &["broken"], &[])["status"],
            0
        );
        assert_eq!(
            call(&runtime, explicit, "InitDevice", &["broken"], &[])["status"],
            0
        );
        assert_eq!(
            call(&runtime, explicit, "执行", &["broken"], &["ok"])["output"],
            serde_json::json!(["1"])
        );
        assert_eq!(
            call(&runtime, explicit, "CloseAllDevices", &[], &[])["status"],
            0
        );
    }
}

#[test]
fn init_probe_close_and_drop_panics_leave_ffi_usable() {
    let runtime = Runtime::new();
    assert_eq!(
        call(&runtime, false, "InitDevice", &["init-panic"], &[])["status"],
        3
    );
    assert_eq!(
        call(&runtime, false, "GetPortNO", &[], &["panic"])["status"],
        3
    );
    for name in [
        "close-panic",
        "drop-panic",
        "close-panic-drop-panic",
        "close-error-drop-panic",
    ] {
        assert_eq!(
            call(&runtime, false, "InitDevice", &[name], &[])["status"],
            0
        );
        assert_eq!(
            call(&runtime, false, "CloseDevice", &[name], &[])["status"],
            3
        );
        assert!(!runtime.get().unwrap().lock().unwrap().is_initialized());
    }
    assert_eq!(
        call(&runtime, false, "GetFrameworkVersion", &[], &[])["status"],
        0
    );
    assert_eq!(
        call(
            &runtime,
            false,
            "InitDevice",
            &["recovered", "Parallel"],
            &[]
        )["status"],
        0
    );
    assert_eq!(
        call(&runtime, false, "CloseAllDevices", &[], &[])["status"],
        0
    );
    let events = CLEANUP.lock().unwrap();
    // Constructors and static probes own their temporary resources; the runtime
    // cannot close an instance that was never returned.
    for name in ["init-panic", "probe"] {
        assert!(events.contains(&format!("drop:{name}")));
        assert!(!events.contains(&format!("close:{name}")));
    }
    for name in ["close-panic", "drop-panic", "close-panic-drop-panic"] {
        assert!(events.contains(&format!("close:{name}")));
        assert!(events.contains(&format!("drop:{name}")));
    }
}

#[test]
fn shared_panic_isolation_covers_all_execution_modes() {
    for mode in ["Sequential", "Parallel", "Broadcast"] {
        let mut runtime = NodeRuntime::<RecoveryNode>::new();
        for name in ["shared-broken", "shared-healthy"] {
            runtime
                .invoke(&request("InitDevice", &[name, mode], &[]))
                .unwrap();
        }
        let second = if mode == "Broadcast" { "panic" } else { "ok" };
        let error = runtime
            .invoke(&request(
                "执行",
                &["shared-broken"],
                &["共享", "panic", second],
            ))
            .unwrap_err();
        assert!(error.to_string().contains("通道 1"));
        assert_eq!(error.code(), "NODE_PANICKED");
        assert!(matches!(
            runtime.invoke(&request("执行", &["shared-broken"], &["ok"])),
            Err(MiresidianError::NodePanicked { .. })
        ));
        assert!(
            runtime
                .invoke(&request("执行", &["shared-healthy"], &["ok"]))
                .is_ok()
        );
        runtime.shutdown().unwrap();
    }
}

#[test]
fn shutdown_attempts_all_nodes_even_when_close_and_drop_panic() {
    let mut runtime = NodeRuntime::<RecoveryNode>::new();
    for name in [
        "shutdown-healthy",
        "shutdown-drop-panic",
        "shutdown-close-panic-drop-panic",
    ] {
        runtime
            .invoke(&request("InitDevice", &[name], &[]))
            .unwrap();
    }
    assert!(runtime.shutdown().is_err());
    assert!(!runtime.is_initialized());
    assert!(runtime.shutdown().is_ok());
    let events = CLEANUP.lock().unwrap();
    for name in [
        "shutdown-healthy",
        "shutdown-drop-panic",
        "shutdown-close-panic-drop-panic",
    ] {
        assert!(events.contains(&format!("close:{name}")));
        assert!(events.contains(&format!("drop:{name}")));
    }
}

#[test]
fn externally_poisoned_runtime_is_cleaned_before_recovery() {
    let runtime = Runtime::new();
    assert_eq!(
        call(&runtime, false, "InitDevice", &["external-poison"], &[])["status"],
        0
    );
    let _ = std::panic::catch_unwind(|| {
        let _guard = runtime.get().unwrap().lock().unwrap();
        panic!("外部调用者污染运行时锁");
    });
    assert!(runtime.get().unwrap().is_poisoned());
    assert_eq!(
        call(&runtime, false, "GetFrameworkVersion", &[], &[])["status"],
        0
    );
    assert!(!runtime.get().unwrap().is_poisoned());
    assert!(!runtime.get().unwrap().lock().unwrap().is_initialized());
    assert_eq!(
        call(&runtime, false, "InitDevice", &["external-recovered"], &[])["status"],
        0
    );
}

fn request(operation: &str, config: &[&str], input: &[&str]) -> Invocation {
    Invocation::new(
        operation,
        config
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
        input
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
    )
}
