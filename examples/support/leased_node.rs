//! 使用现有 lease 标注，初始化直接打开独占资源；不手写共享代理。
use miresidian::{MiresidianError, MiresidianResult};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    time::Duration,
};

#[miresidian::node("自动租约资源", instance(mode = multiple, binding = handle, execution = sequential),
    lease(activation = selectable, resource = "resource", configuration = ["setting"]))]
pub struct LeasedNode {
    _file: File,
    events: File,
    calls: usize,
}
impl LeasedNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("资源")] resource: String,
        #[miresidian::config("配置")] setting: u32,
    ) -> MiresidianResult<(Self, String)> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&resource)
            .map_err(io_error)?;
        file.try_lock()
            .map_err(|_| MiresidianError::Failure("资源被重复打开".into()))?;
        let mut events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{resource}.events"))
            .map_err(io_error)?;
        writeln!(events, "open:{setting}").map_err(io_error)?;
        Ok((
            Self {
                _file: file,
                events,
                calls: 0,
            },
            format!("connected:{setting}"),
        ))
    }
    #[miresidian::operation("查询")]
    fn query(
        &mut self, #[miresidian::input("消息")] message: String
    ) -> MiresidianResult<String> {
        writeln!(self.events, "begin:{message}").map_err(io_error)?;
        std::thread::sleep(Duration::from_millis(10));
        self.calls += 1;
        writeln!(self.events, "end:{message}").map_err(io_error)?;
        Ok(format!("{}:{message}", self.calls))
    }
    #[miresidian::operation("异步查询")]
    async fn async_query(&mut self) -> MiresidianResult<usize> {
        tokio::time::sleep(Duration::from_millis(1)).await;
        self.calls += 1;
        Ok(self.calls)
    }
    #[miresidian::operation("参数错误")]
    fn invalid(&mut self) -> MiresidianResult<()> {
        Err(MiresidianError::InvalidArgument {
            field: "消息".into(),
            message: "测试错误".into(),
        }
        .with_context("远端"))
    }
    #[miresidian::operation("异常")]
    fn panic(&mut self) -> MiresidianResult<()> {
        panic!("owner panic")
    }
    #[miresidian::operation]
    fn close_device(&mut self) -> MiresidianResult<()> {
        writeln!(self.events, "close").map_err(io_error)
    }
}
impl Drop for LeasedNode {
    fn drop(&mut self) {
        let _ = writeln!(self.events, "drop");
    }
}
fn io_error(error: std::io::Error) -> MiresidianError {
    MiresidianError::Failure(error.to_string())
}
