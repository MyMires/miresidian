//! 框架内部的模块加载寿命夹具，用阻塞任务验证宿主释放引用后的线程与模块保护。
//! module_probe_* 是原生加载器验证专用导出，不能当作节点公共入口或交付头文件。
//! node 宏仍生成当前 C ABI v8；私有探针只负责启动、观察和关闭执行器。
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
// 无设备资源的节点用于同时启动并清理生命周期池，使验证覆盖两类执行器。
// 私有探针宿主不调用标准节点导出；资源与设备行为不在本夹具验收范围内。
#[miresidian::node("模块加载验证夹具")]
struct LifecycleProbe;
impl LifecycleProbe {
    #[miresidian::operation]
    fn init_device() -> miresidian::MiresidianResult<Self> {
        Ok(Self)
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> miresidian::MiresidianResult<()> {
        Ok(())
    }
}

static STATUS: AtomicU32 = AtomicU32::new(0);

#[unsafe(no_mangle)]
/// 启动一次模拟工作；0 表示成功，1 表示已启动、初始化/关闭失败或捕获到 panic。
/// 宿主在本次调用期间须保留模块引用，并通过 status 等待任务结束后再尝试 stop。
pub extern "C" fn module_probe_start() -> u32 {
    std::panic::catch_unwind(|| {
        if STATUS.load(Ordering::Acquire) != 0 {
            return 1;
        }
        let runtime = miresidian::NodeRuntime::<LifecycleProbe>::new();
        if runtime
            .invoke(&miresidian::Invocation::new("InitDevice", vec![], vec![]))
            .is_err()
        {
            return 1;
        }
        if runtime.shutdown().is_err() {
            return 1;
        }
        drop(runtime);
        let result = miresidian::__private::block_on(async {
            tokio::task::spawn_blocking(|| {
                STATUS.store(1, Ordering::Release);
                std::thread::sleep(Duration::from_millis(150));
                STATUS.store(2, Ordering::Release);
            });
            while STATUS.load(Ordering::Acquire) == 0 {
                tokio::task::yield_now().await;
            }
            Ok(())
        });
        u32::from(result.is_err())
    })
    .unwrap_or(1)
}
#[unsafe(no_mangle)]
/// 读取模拟工作状态：0 尚未开始，1 正在等待，2 已结束；不等待也不消费状态。
pub extern "C" fn module_probe_status() -> u32 {
    STATUS.load(Ordering::Acquire)
}
#[unsafe(no_mangle)]
/// 尝试回收两类执行器；0 表示成功，1 表示仍有工作/引用、关闭错误或 panic。
/// 这是测试探针，不增加节点公共 ABI 的执行器关闭能力。
pub extern "C" fn module_probe_stop() -> u32 {
    std::panic::catch_unwind(|| u32::from(miresidian::shutdown_executor().is_err())).unwrap_or(1)
}
