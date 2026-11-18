//! Internal registry-node fixture copied only into the test's temporary project.
use miresidian::{MiresidianError, MiresidianResult};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

#[miresidian::node("TestFlow policy fixture", instance(mode = multiple, binding = handle, execution = selectable, default_execution = sequential), lease(activation = selectable, resource = "resource", configuration = ["setting"]))]
struct PolicyNode {
    _file: File,
    events: File,
    resource: PathBuf,
    calls: usize,
}
impl PolicyNode {
    #[miresidian::operation]
    fn init_device(
        #[miresidian::config("资源")] resource: String,
        #[miresidian::config("配置")] setting: u32,
        #[miresidian::input("令牌:")] token: String,
    ) -> MiresidianResult<(Self, String)> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&resource)
            .map_err(error)?;
        file.try_lock()
            .map_err(|e| MiresidianError::Failure(e.to_string()))?;
        let mut events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{resource}.events"))
            .map_err(error)?;
        writeln!(events, "open:{setting}:{token}").map_err(error)?;
        let node = Self {
            _file: file,
            events,
            resource: resource.into(),
            calls: 0,
        };
        if token == "fail" {
            return Err(MiresidianError::Failure("init failed".into()));
        }
        assert_ne!(token, "panic", "init panic");
        Ok((node, token))
    }
    #[miresidian::operation]
    fn record(
        &mut self,
        #[miresidian::input("命令")] command: String,
    ) -> MiresidianResult<(usize, String)> {
        writeln!(self.events, "begin:{command}").map_err(error)?;
        self.calls += 1;
        if command == "hold" {
            std::fs::write(self.resource.with_extension("entered"), b"entered").map_err(error)?;
            let deadline = Instant::now() + Duration::from_secs(10);
            while !self.resource.with_extension("release").exists() {
                if Instant::now() > deadline {
                    return Err(MiresidianError::Failure("test barrier deadline".into()));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        assert_ne!(command, "panic", "business panic");
        writeln!(self.events, "end:{command}").map_err(error)?;
        if command == "fail" {
            return Err(MiresidianError::Failure("business failed".into()));
        }
        Ok((self.calls, command))
    }
    #[miresidian::operation]
    fn close_device(
        &mut self,
        #[miresidian::input("关闭参数:")] reason: String,
    ) -> MiresidianResult<()> {
        writeln!(self.events, "close:{reason}").map_err(error)?;
        assert_ne!(reason, "panic", "close panic");
        if reason == "fail" {
            return Err(MiresidianError::Failure("close failed".into()));
        }
        Ok(())
    }
}
impl Drop for PolicyNode {
    fn drop(&mut self) {
        let _ = writeln!(self.events, "drop");
    }
}
fn error(e: std::io::Error) -> MiresidianError {
    MiresidianError::Failure(e.to_string())
}
