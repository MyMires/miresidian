//! Task snapshots are independent of submitting and polling threads.
use crate::{CallContext, InvocationOutput, MiresidianResult};
use serde::Serialize;
use std::sync::Mutex;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}
#[derive(Clone, Debug, Serialize)]
pub struct TaskSnapshot {
    pub id: u64,
    pub state: TaskState,
    pub cancellation_requested: bool,
    pub code: &'static str,
    pub message: String,
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

/// Keep per-item batch envelopes in the same field order as normal responses.
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
