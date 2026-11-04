#![cfg(feature = "local-ipc")]
//! 使用真正的独占文件句柄模拟串口；第二次工厂执行会因独占锁失败。
use miresidian_lease::{
    LeaseConfiguration, LeaseError, ResourceKey,
    shared::{ResourceHandler, SharedResource},
};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, Write},
    process::{Command, Stdio},
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const PROTOCOL: &str = "test-exclusive-device/1";
fn key(name: &str) -> ResourceKey {
    ResourceKey::new(format!(
        "shared-resource-test-{}-{name}",
        std::process::id()
    ))
    .unwrap()
}
fn config(value: u8) -> LeaseConfiguration {
    LeaseConfiguration::new(vec![value]).unwrap()
}

struct Device {
    _file: File,
    closed: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
}
impl Device {
    fn open(
        key: &ResourceKey,
        opened: &AtomicUsize,
        closed: Arc<AtomicUsize>,
        calls: Arc<AtomicUsize>,
    ) -> Result<Self, LeaseError> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(std::env::temp_dir().join(key.as_str()))
            .unwrap();
        file.try_lock()
            .map_err(|_| LeaseError::LocalIpc("exclusive resource opened twice".into()))?;
        opened.fetch_add(1, Ordering::SeqCst);
        Ok(Self {
            _file: file,
            closed,
            calls,
        })
    }
}
impl ResourceHandler for Device {
    fn transact(&mut self, request: &[u8]) -> Result<Vec<u8>, LeaseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match request {
            b"error" => Err(LeaseError::LocalIpc("device error".into())),
            b"panic" => panic!("device panic"),
            b"slow" => {
                std::thread::sleep(Duration::from_millis(100));
                Ok(request.to_vec())
            }
            _ => Ok(request.to_vec()),
        }
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        self.closed.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn concurrent_clients_open_once_and_last_close_releases_the_resource() {
    let resource = key("concurrent");
    let opened = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(8));
    let mut workers = Vec::new();
    for _ in 0..8 {
        let (resource, opened, closed, calls, barrier) = (
            resource.clone(),
            opened.clone(),
            closed.clone(),
            calls.clone(),
            barrier.clone(),
        );
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            SharedResource::acquire(resource.clone(), config(1), PROTOCOL, || {
                Device::open(&resource, &opened, closed, calls)
            })
            .unwrap()
        }));
    }
    let mut clients: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(opened.load(Ordering::SeqCst), 1);
    assert!(matches!(
        SharedResource::acquire(
            resource.clone(),
            config(2),
            PROTOCOL,
            || -> Result<Device, LeaseError> { panic!("incompatible factory must not run") }
        ),
        Err(LeaseError::IncompatibleConfiguration)
    ));
    assert!(
        SharedResource::acquire(
            resource.clone(),
            config(1),
            "other-protocol",
            || -> Result<Device, LeaseError> { panic!("wrong protocol factory must not run") }
        )
        .is_err()
    );
    for _ in 0..7 {
        clients.pop().unwrap().close().unwrap();
    }
    assert_eq!(closed.load(Ordering::SeqCst), 0);
    let mut last = clients.pop().unwrap();
    assert_eq!(
        last.transact(b"alive", Duration::from_secs(1)).unwrap(),
        b"alive"
    );
    last.close().unwrap();
    last.close().unwrap();
    assert!(matches!(
        last.transact(b"closed", Duration::from_secs(1)),
        Err(LeaseError::Invalidated)
    ));
    assert_eq!(closed.load(Ordering::SeqCst), 1);
    let mut replacement = SharedResource::acquire(resource.clone(), config(2), PROTOCOL, || {
        Device::open(&resource, &opened, closed.clone(), calls.clone())
    })
    .unwrap();
    replacement.close().unwrap();
    assert_eq!(opened.load(Ordering::SeqCst), 2);
    assert_eq!(closed.load(Ordering::SeqCst), 2);
}

#[test]
fn complete_transactions_are_serialized_across_independent_clients() {
    let active = Arc::new(AtomicUsize::new(0));
    let resource = key("transactions");
    let active_handler = active.clone();
    let first = SharedResource::acquire(resource.clone(), config(1), PROTOCOL, || {
        Ok(move |request: &[u8]| {
            assert_eq!(active_handler.fetch_add(1, Ordering::SeqCst), 0);
            std::thread::sleep(Duration::from_millis(20));
            assert_eq!(active_handler.fetch_sub(1, Ordering::SeqCst), 1);
            Ok(request.to_vec())
        })
    })
    .unwrap();
    let second = SharedResource::acquire(
        resource,
        config(1),
        PROTOCOL,
        || -> Result<Device, LeaseError> { panic!("second factory ran") },
    )
    .unwrap();
    let workers: Vec<_> = [first, second]
        .into_iter()
        .enumerate()
        .map(|(i, mut client)| {
            std::thread::spawn(move || {
                for _ in 0..3 {
                    assert_eq!(
                        client.transact(&[i as u8], Duration::from_secs(1)).unwrap(),
                        [i as u8]
                    );
                }
                client.close().unwrap();
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

#[test]
fn device_errors_are_returned_without_replay_and_panics_fault_all_clients() {
    let resource = key("errors");
    let opened = AtomicUsize::new(0);
    let calls = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let mut first = SharedResource::acquire(resource.clone(), config(1), PROTOCOL, || {
        Device::open(&resource, &opened, closed.clone(), calls.clone())
    })
    .unwrap();
    let mut second = SharedResource::acquire(
        resource,
        config(1),
        PROTOCOL,
        || -> Result<Device, LeaseError> { panic!("second factory ran") },
    )
    .unwrap();
    assert_eq!(
        first.transact(b"error", Duration::from_secs(1)),
        Err(LeaseError::LocalIpc("device error".into()))
    );
    assert_eq!(
        first.transact(b"ok", Duration::from_secs(1)).unwrap(),
        b"ok"
    );
    assert!(matches!(
        first.transact(b"panic", Duration::from_secs(1)),
        Err(LeaseError::Invalidated)
    ));
    assert!(matches!(
        second.transact(b"after", Duration::from_secs(1)),
        Err(LeaseError::Invalidated)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let _ = first.close();
    let _ = second.close();
}

#[test]
fn timeout_invalidates_only_the_client_and_does_not_replay_the_transaction() {
    let resource = key("timeout");
    let opened = AtomicUsize::new(0);
    let calls = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let mut first = SharedResource::acquire(resource.clone(), config(1), PROTOCOL, || {
        Device::open(&resource, &opened, closed.clone(), calls.clone())
    })
    .unwrap();
    let mut second = SharedResource::acquire(
        resource,
        config(1),
        PROTOCOL,
        || -> Result<Device, LeaseError> { panic!("second factory ran") },
    )
    .unwrap();
    assert!(first.transact(b"slow", Duration::from_millis(10)).is_err());
    assert!(matches!(
        first.transact(b"replay", Duration::from_secs(1)),
        Err(LeaseError::Invalidated)
    ));
    assert_eq!(
        second
            .transact(b"still-alive", Duration::from_secs(1))
            .unwrap(),
        b"still-alive"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    first.close().unwrap();
    second.close().unwrap();
}

#[test]
fn failed_factory_releases_ownership_for_a_later_initializer() {
    let resource = key("factory");
    assert!(
        SharedResource::acquire(
            resource.clone(),
            config(1),
            PROTOCOL,
            || -> Result<Device, LeaseError> { Err(LeaseError::LocalIpc("open failed".into())) }
        )
        .is_err()
    );
    let mut resource = SharedResource::acquire(resource, config(2), PROTOCOL, || {
        Ok(|bytes: &[u8]| Ok(bytes.to_vec()))
    })
    .unwrap();
    assert_eq!(
        resource
            .transact(b"retry-init", Duration::from_secs(1))
            .unwrap(),
        b"retry-init"
    );
    resource.close().unwrap();
}

#[test]
fn independent_process_uses_the_existing_owner_without_executing_its_factory() {
    let resource = key("process");
    let opened = AtomicUsize::new(0);
    let calls = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let mut owner = SharedResource::acquire(resource.clone(), config(1), PROTOCOL, || {
        Device::open(&resource, &opened, closed.clone(), calls.clone())
    })
    .unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "independent_process_worker", "--nocapture"])
        .env("MIRESIDIAN_SHARED_TEST_RESOURCE", resource.as_str())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(opened.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(closed.load(Ordering::SeqCst), 0);
    owner.close().unwrap();
    assert_eq!(closed.load(Ordering::SeqCst), 1);
}

#[test]
fn independent_process_worker() {
    let Ok(resource) = std::env::var("MIRESIDIAN_SHARED_TEST_RESOURCE") else {
        return;
    };
    if std::env::var_os("MIRESIDIAN_SHARED_OWNER_WORKER").is_some() {
        let key = ResourceKey::new(resource).unwrap();
        let _owner = SharedResource::acquire(key.clone(), config(1), PROTOCOL, || {
            Device::open(
                &key,
                &AtomicUsize::new(0),
                Arc::new(AtomicUsize::new(0)),
                Arc::new(AtomicUsize::new(0)),
            )
        })
        .unwrap();
        println!("RESOURCE_READY");
        std::io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    let mut client = SharedResource::acquire(
        ResourceKey::new(resource).unwrap(),
        config(1),
        PROTOCOL,
        || -> Result<Device, LeaseError> {
            panic!("a different process attempted to open the exclusive device")
        },
    )
    .unwrap();
    assert_eq!(
        client.transact(b"child", Duration::from_secs(1)).unwrap(),
        b"child"
    );
    client.close().unwrap();
}

#[test]
fn owner_process_death_invalidates_existing_clients_without_reopening_or_replay() {
    // 先在父进程启动租约协调服务，单独终止物理资源所有者。
    let mut coordinator =
        miresidian_lease::local_ipc::LocalLease::acquire(key("coordinator"), config(1)).unwrap();
    let resource = key("owner-death");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "independent_process_worker", "--nocapture"])
        .env("MIRESIDIAN_SHARED_TEST_RESOURCE", resource.as_str())
        .env("MIRESIDIAN_SHARED_OWNER_WORKER", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = std::io::BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert_ne!(
            reader.read_line(&mut line).unwrap(),
            0,
            "owner exited before ready"
        );
        if line.contains("RESOURCE_READY") {
            break;
        }
    }
    let mut client = SharedResource::acquire(
        resource.clone(),
        config(1),
        PROTOCOL,
        || -> Result<Device, LeaseError> { panic!("parent must not open existing resource") },
    )
    .unwrap();
    assert_eq!(
        client.transact(b"before", Duration::from_secs(1)).unwrap(),
        b"before"
    );
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(client.transact(b"after", Duration::from_secs(1)).is_err());
    assert!(matches!(
        client.transact(b"retry", Duration::from_secs(1)),
        Err(LeaseError::Invalidated)
    ));
    client.close().unwrap();
    coordinator.validate().unwrap();
    let opened = AtomicUsize::new(0);
    let mut replacement = SharedResource::acquire(resource.clone(), config(1), PROTOCOL, || {
        Device::open(
            &resource,
            &opened,
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        )
    })
    .unwrap();
    assert_eq!(
        replacement
            .transact(b"new-init", Duration::from_secs(1))
            .unwrap(),
        b"new-init"
    );
    replacement.close().unwrap();
    assert_eq!(opened.load(Ordering::SeqCst), 1);
    coordinator.release().unwrap();
}
