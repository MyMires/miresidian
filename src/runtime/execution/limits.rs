//! Runtime request admission and execution limits.
use crate::{MiresidianError, MiresidianResult};
/// Limits apply to one runtime. Pending requests acquire instance execution
/// rights before global running permits, so one busy instance cannot occupy
/// all blocking workers while waiting for its own lock.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimits {
    pub max_running: usize,
    pub max_pending: usize,
    pub max_pending_per_instance: usize,
    pub max_batch: usize,
}
impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_running: 32,
            max_pending: 1024,
            max_pending_per_instance: 64,
            max_batch: 256,
        }
    }
}
impl ExecutionLimits {
    pub(crate) fn validate(self) -> MiresidianResult<Self> {
        if [
            self.max_running,
            self.max_pending,
            self.max_pending_per_instance,
            self.max_batch,
        ]
        .contains(&0)
            || self.max_running > 65536
            || self.max_pending > 65536
            || self.max_pending_per_instance > 65536
            || self.max_batch > 65536
        {
            return Err(MiresidianError::InvalidArgument {
                field: "ExecutionLimits".into(),
                message: "限制须为正数；所有限制 <= 65536".into(),
            });
        }
        Ok(self)
    }
}
