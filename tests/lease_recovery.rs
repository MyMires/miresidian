#![cfg(feature = "lease")]
//! 终止真实服务宿主进程，验证跨代配置锁、租约失效和节点恢复。
use miresidian::{
    Invocation, MiresidianResult, NodeRuntime,
    lease::{LeaseConfiguration, LeaseError, ResourceKey, local_ipc::LocalLease},
};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

#[miresidian::node("租约恢复测试", lease(activation = required, resource = "resource", configuration = ["setting"]))]
struct LeasedNode {
    calls: usize,
}

impl LeasedNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("资源")] resource: String,
        #[miresidian::config("配置")] setting: u32,
    ) -> MiresidianResult<Self> {
        let _ = (resource, setting);
        Ok(Self { calls: 0 })
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        Ok(())
    }
    #[miresidian::operation("计数")]
    fn count(&mut self) -> MiresidianResult<usize> {
        self.calls += 1;
        Ok(self.calls)
    }
}

fn acquire(resource: &str, configuration: u8) -> Result<LocalLease, LeaseError> {
    LocalLease::acquire(
        ResourceKey::new(resource).unwrap(),
        LeaseConfiguration::new(vec![configuration]).unwrap(),
    )
}

// 宏将配置值编码为带名称的帧；手工客户端也使用同一规范化配置。
fn node_configuration() -> LeaseConfiguration {
    miresidian::lease::LeaseRequest::from_parameters(
        vec![
            miresidian::lease::LeaseParameter::new("resource", "unused"),
            miresidian::lease::LeaseParameter::new("setting", "1"),
        ],
        "resource",
        &["setting"],
    )
    .unwrap()
    .configuration()
    .clone()
}

fn compatible(resource: &str) -> LocalLease {
    LocalLease::acquire(ResourceKey::new(resource).unwrap(), node_configuration()).unwrap()
}

struct Session {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
}

impl Session {
    fn start(role: &str, resource: &str) -> Self {
        let namespace = if role == "conflicting-namespace" {
            format!("{resource}:other")
        } else {
            resource.to_owned()
        };
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "lease_process_helper",
                "--nocapture",
            ])
            .env("MIRESIDIAN_LEASE_TEST_ROLE", role)
            .env("MIRESIDIAN_LEASE_TEST_RESOURCE", resource)
            .env("MIRESIDIAN_LEASE_SERVICE_NAMESPACE", namespace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            lines,
        }
    }
    fn receive(&self) -> String {
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(15))
                .expect("子进程必须在期限内响应");
            if let Some(message) = line.strip_prefix("LEASE_TEST:") {
                return message.to_owned();
            }
        }
    }
    fn command(&mut self, command: &str) -> String {
        writeln!(self.input, "{command}").unwrap();
        self.input.flush().unwrap();
        self.receive()
    }
    fn terminate(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn service_owner_exit_preserves_configuration_and_invalidates_runtime() {
    let resource = format!("lease-recovery:{}", std::process::id());
    // 场景及其手工客户端均在独立子进程运行，环境不会污染并行测试或真实服务。
    let mut coordinator = Session::start("coordinator", &resource);
    assert_eq!(coordinator.receive(), "completed");
    assert!(coordinator.child.wait().unwrap().success());
}

fn run_owner_exit_scenario(resource: &str) {
    let mut owner = Session::start("owner", resource);
    let generation = owner.receive();
    let mut survivor = Session::start("runtime", resource);
    assert_eq!(survivor.receive(), "ready");
    assert_eq!(survivor.command("invoke"), "ok:1");
    owner.terminate();
    let mut competitor = Session::start("conflicting-namespace", resource);
    assert_eq!(competitor.receive(), "incompatible");
    assert!(competitor.child.wait().unwrap().success());

    assert!(survivor.child.try_wait().unwrap().is_none());
    assert!(matches!(
        acquire(resource, 9),
        Err(LeaseError::IncompatibleConfiguration)
    ));
    let mut replacement = compatible(resource);
    replacement.validate().unwrap();
    assert_ne!(generation, format!("ready:{:?}", replacement.generation()));

    assert!(survivor.command("invoke").contains("租约已失效"));
    assert!(survivor.command("invoke").contains("租约已失效"));
    assert!(survivor.command("init").contains("租约已失效"));
    // 失效后仍持有配置锁，直到资源关闭。
    replacement.release().unwrap();
    assert!(matches!(
        acquire(resource, 9),
        Err(LeaseError::IncompatibleConfiguration)
    ));
    assert_eq!(survivor.command("close"), "closed:false");

    let mut changed = acquire(resource, 9).expect("全部设备关闭后允许新配置");
    changed.release().unwrap();
    assert_eq!(survivor.command("init"), "ok");
    assert_eq!(survivor.command("invoke"), "ok:1");
    assert_eq!(survivor.command("close"), "closed:false");
}

#[test]
#[ignore = "仅由多进程回归测试作为子进程启动"]
fn lease_process_helper() {
    let role = std::env::var("MIRESIDIAN_LEASE_TEST_ROLE").unwrap();
    let resource = std::env::var("MIRESIDIAN_LEASE_TEST_RESOURCE").unwrap();
    if role == "coordinator" {
        run_owner_exit_scenario(&resource);
        emit("completed");
        return;
    }
    if role == "conflicting-namespace" {
        assert!(matches!(
            acquire(&resource, 9),
            Err(LeaseError::IncompatibleConfiguration)
        ));
        emit("incompatible");
        return;
    }
    if role == "owner" {
        let lease = compatible(&resource);
        emit(&format!("ready:{:?}", lease.generation()));
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        return;
    }
    let mut runtime = NodeRuntime::<LeasedNode>::new();
    let config = vec![resource, "1".to_owned()];
    runtime
        .invoke(&Invocation::new("InitDevice", config.clone(), vec![]))
        .unwrap();
    emit("ready");
    for line in std::io::stdin().lock().lines() {
        match line.unwrap().as_str() {
            "invoke" => match runtime.invoke(&Invocation::new("计数", config.clone(), vec![])) {
                Ok(output) => emit(&format!("ok:{}", output.as_slice()[0])),
                Err(error) => emit(&error.to_string()),
            },
            "init" => {
                match runtime.invoke(&Invocation::new("InitDevice", config.clone(), vec![])) {
                    Ok(_) => emit("ok"),
                    Err(error) => emit(&error.to_string()),
                }
            }
            "close" => {
                let _ = runtime.shutdown();
                emit(&format!("closed:{}", runtime.is_initialized()));
            }
            _ => panic!("不支持的测试命令"),
        }
    }
}

fn emit(message: &str) {
    println!("LEASE_TEST:{message}");
    std::io::stdout().flush().unwrap();
}

#[test]
fn singleton_replacement_releases_old_lease_before_requesting_new_configuration() {
    let resource = format!("singleton-lease-replacement:{}", std::process::id());
    let mut runtime = NodeRuntime::<LeasedNode>::new();
    let request = |operation: &str, setting: &str| {
        Invocation::new(
            operation,
            vec![resource.clone(), setting.to_owned()],
            vec![],
        )
    };
    runtime.invoke(&request("InitDevice", "1")).unwrap();
    assert_eq!(
        runtime.invoke(&request("计数", "1")).unwrap().as_slice(),
        ["1"]
    );
    assert!(runtime.invoke(&request("InitDevice", "invalid")).is_err());
    assert_eq!(
        runtime.invoke(&request("计数", "1")).unwrap().as_slice(),
        ["2"]
    );
    runtime
        .invoke(&request("InitDevice", "2"))
        .expect("旧对象及其配置锁必须先释放");
    assert_eq!(
        runtime.invoke(&request("计数", "2")).unwrap().as_slice(),
        ["1"]
    );
    runtime.shutdown().unwrap();
}
