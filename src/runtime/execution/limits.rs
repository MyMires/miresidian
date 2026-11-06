//! 单运行时的请求接收、执行与批次限额；超限拒绝，不通过无限排队吸收请求。
use crate::{MiresidianError, MiresidianResult};
/// 一个运行时及其 Clone 共用的容量配置，各值须在 1..=65536。
///
/// 请求先等待实例执行权，再取得全局运行许可，避免同一繁忙实例的等待者耗尽全局预算。
/// 已开始和等待中的请求均计入 pending；限制不等同于执行器线程数。
#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimits {
    /// 全局同时执行请求的许可上限，默认 32；不包含仅等待实例执行权的请求。
    pub max_running: usize,
    /// 全运行时已接收且尚未结束的请求上限，默认 1024，含正在执行的请求。
    pub max_pending: usize,
    /// 单实例的在途及等待请求上限，默认 64。
    pub max_pending_per_instance: usize,
    /// 单次多通道或显式跨实例批次的项数上限，默认 256。
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
