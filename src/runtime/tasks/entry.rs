//! 任务快照复制结果与状态，不借用提交或查询线程的响应缓存，可跨线程读取。
use crate::{CallContext, InvocationOutput, MiresidianResult};
use serde::Serialize;
use std::sync::Mutex;
/// 查询时观察到的任务状态；终态以实际执行结果决定，不能仅从取消信号推导。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// 尚未标记开始处理；排队取消仍可短暂保留此状态及取消标志。
    Queued,
    /// 已开始处理且尚无最终结果。
    Running,
    /// 已开始并收到取消或截止时间到达，正在等待业务安全结束。
    Cancelling,
    /// 最终成功；即使请求过取消，业务仍可能返回成功。
    Succeeded,
    /// 最终失败，且不是取消或截止时间错误。
    Failed,
    /// 最终以 REQUEST_CANCELLED 错误结束。
    Cancelled,
    /// 最终以 DEADLINE_EXCEEDED 错误结束。
    TimedOut,
}
/// 可跨线程持有的任务状态和结果副本；读取不释放服务容量，后续快照可能继续变化。
#[derive(Clone, Debug, Serialize)]
pub struct TaskSnapshot {
    /// 所属服务内单调分配、不复用的任务 ID。
    pub id: u64,
    /// 本次读取时的状态，最终结果优先于取消信号。
    pub state: TaskState,
    /// 是否曾显式请求取消；不等于已经停止或截止时间已到。
    pub cancellation_requested: bool,
    /// 未完成为 PENDING，完成后为 OK 或稳定错误代码。
    pub code: &'static str,
    /// 错误说明；无错误时为空，不应通过解析文本判断错误类型。
    pub message: String,
    /// 成功业务输出；批次的每个槽位是对应请求的完整 JSON 响应。
    pub output: Vec<String>,
}
pub(super) struct TaskEntry {
    pub context: CallContext,
    pub result: Mutex<Option<MiresidianResult<InvocationOutput>>>,
    pub handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub changed: tokio::sync::Notify,
    id: u64,
}
impl TaskEntry {
    pub fn new(id: u64, context: CallContext) -> Self {
        Self {
            id,
            context,
            result: Mutex::new(None),
            handle: Mutex::new(None),
            changed: tokio::sync::Notify::new(),
        }
    }
    pub fn snapshot(&self) -> TaskSnapshot {
        let result = self.result.lock().unwrap_or_else(|e| e.into_inner());
        let (state, code, message, output) = match result.as_ref() {
            Some(Ok(output)) => (
                TaskState::Succeeded,
                "OK",
                String::new(),
                output.as_slice().to_vec(),
            ),
            Some(Err(error)) => {
                let state = if reason(error, "REQUEST_CANCELLED") {
                    TaskState::Cancelled
                } else if reason(error, "DEADLINE_EXCEEDED") {
                    TaskState::TimedOut
                } else {
                    TaskState::Failed
                };
                (state, error.code(), error.to_string(), Vec::new())
            }
            None => {
                let state = if self.context.is_started() {
                    if self.context.is_cancelled() || self.context.is_expired() {
                        TaskState::Cancelling
                    } else {
                        TaskState::Running
                    }
                } else {
                    TaskState::Queued
                };
                (state, "PENDING", String::new(), Vec::new())
            }
        };
        TaskSnapshot {
            id: self.id,
            state,
            cancellation_requested: self.context.is_cancelled(),
            code,
            message,
            output,
        }
    }
}

fn reason(error: &crate::MiresidianError, tag: &str) -> bool {
    match error {
        crate::MiresidianError::Context { context, source } => {
            context == tag || reason(source, tag)
        }
        _ => false,
    }
}

/// 把跨实例批次的每项结果编码为独立 JSON 响应，字段顺序与普通响应保持一致。
pub(super) fn batch_result(result: MiresidianResult<InvocationOutput>) -> MiresidianResult<String> {
    #[derive(Serialize)]
    struct ResultEnvelope {
        status: i32,
        code: &'static str,
        message: String,
        output: Vec<String>,
    }
    let envelope = match result {
        Ok(output) => ResultEnvelope {
            status: 0,
            code: "OK",
            message: String::new(),
            output: output.into_values(),
        },
        Err(error) => ResultEnvelope {
            status: match error.category() {
                crate::ErrorCategory::InvalidArgument => 1,
                crate::ErrorCategory::OperationFailed => 2,
                crate::ErrorCategory::InternalError => 3,
            },
            code: error.code(),
            message: error.to_string(),
            output: Vec::new(),
        },
    };
    serde_json::to_string(&envelope).map_err(|e| crate::MiresidianError::Internal(e.to_string()))
}
