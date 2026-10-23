#![cfg(feature = "lease")]

use std::sync::atomic::{AtomicUsize, Ordering};

use miresidian::{
    CLOSE_DEVICE_OPERATION, INIT_DEVICE_OPERATION, Invocation, MiresidianError, MiresidianResult,
    NodeRuntime,
};

static ACQUIRE_COUNT: AtomicUsize = AtomicUsize::new(0);
static RELEASE_COUNT: AtomicUsize = AtomicUsize::new(0);
static INIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static CLOSE_COUNT: AtomicUsize = AtomicUsize::new(0);

#[miresidian::node("租约宏测试节点")]
#[derive(Default)]
struct LeaseMacroNode {
    held: bool,
}

#[miresidian::node_impl]
#[miresidian::lease]
impl LeaseMacroNode {
    fn lease_acquire(&mut self, _config: &[String], _input: &[String]) -> MiresidianResult<()> {
        self.held = true;
        ACQUIRE_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn lease_release(&mut self) -> MiresidianResult<()> {
        if !self.held {
            return Err(MiresidianError::Failure("租约未持有".to_owned()));
        }
        self.held = false;
        RELEASE_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    #[miresidian::output("状态")]
    fn init_device(&mut self) -> MiresidianResult<String> {
        if !self.held {
            return Err(MiresidianError::Failure("初始化前未取得租约".to_owned()));
        }
        INIT_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok("已初始化".to_owned())
    }

    fn close_device(&mut self) -> MiresidianResult<()> {
        if !self.held {
            return Err(MiresidianError::Failure("关闭前租约已释放".to_owned()));
        }
        CLOSE_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn request(operation: &str) -> Invocation {
    Invocation::new(operation, Vec::new(), Vec::new())
}

#[test]
fn lease_attribute_wraps_node_lifecycle_only_when_feature_is_enabled() {
    ACQUIRE_COUNT.store(0, Ordering::SeqCst);
    RELEASE_COUNT.store(0, Ordering::SeqCst);
    INIT_COUNT.store(0, Ordering::SeqCst);
    CLOSE_COUNT.store(0, Ordering::SeqCst);

    let mut runtime = NodeRuntime::<LeaseMacroNode>::new();
    assert_eq!(
        runtime
            .invoke(&request(INIT_DEVICE_OPERATION))
            .expect("租约获取后应初始化成功")
            .as_slice(),
        ["已初始化"]
    );
    assert_eq!(ACQUIRE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(INIT_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(RELEASE_COUNT.load(Ordering::SeqCst), 0);

    runtime
        .invoke(&request(CLOSE_DEVICE_OPERATION))
        .expect("关闭时应释放租约");
    assert_eq!(CLOSE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(RELEASE_COUNT.load(Ordering::SeqCst), 1);
}
