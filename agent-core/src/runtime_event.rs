use serde::{Deserialize, Serialize};

use crate::model::{new_id, Id, InputEnvelope, ToolCall, ToolResult, Usage};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeEvent {
    TurnStarted {
        event_id: Id,
        turn_id: Id,
        input: InputEnvelope,
    },
    ModelTextDelta {
        event_id: Id,
        epoch: u64,
        delta: String,
    },
    ModelReasoningDelta {
        event_id: Id,
        epoch: u64,
        delta: String,
    },
    ModelToolCall {
        event_id: Id,
        epoch: u64,
        call: ToolCall,
    },
    ModelCompleted {
        event_id: Id,
        epoch: u64,
        usage: Option<Usage>,
    },
    ToolDispatched {
        event_id: Id,
        epoch: u64,
        call_id: Id,
    },
    ToolStdoutDelta {
        event_id: Id,
        epoch: u64,
        call_id: Id,
        delta: String,
    },
    ToolStderrDelta {
        event_id: Id,
        epoch: u64,
        call_id: Id,
        delta: String,
    },
    ToolExit {
        event_id: Id,
        epoch: u64,
        call_id: Id,
        exit_code: Option<i32>,
        duration_ms: u64,
    },
    ToolResultOk {
        event_id: Id,
        epoch: u64,
        result: ToolResult,
    },
    ToolResultErr {
        event_id: Id,
        epoch: u64,
        result: ToolResult,
    },
    InputInjected {
        event_id: Id,
        input: InputEnvelope,
    },
    RetryTimerFired {
        event_id: Id,
        next_epoch: u64,
    },
    TransientError {
        event_id: Id,
        epoch: u64,
        message: String,
        retry_after_ms: Option<u64>,
    },
    FatalError {
        event_id: Id,
        message: String,
    },
    CancelRequested {
        event_id: Id,
        reason: Option<String>,
    },
}

impl RuntimeEvent {
    pub fn id(&self) -> &str {
        match self {
            RuntimeEvent::TurnStarted { event_id, .. }
            | RuntimeEvent::ModelTextDelta { event_id, .. }
            | RuntimeEvent::ModelReasoningDelta { event_id, .. }
            | RuntimeEvent::ModelToolCall { event_id, .. }
            | RuntimeEvent::ModelCompleted { event_id, .. }
            | RuntimeEvent::ToolDispatched { event_id, .. }
            | RuntimeEvent::ToolStdoutDelta { event_id, .. }
            | RuntimeEvent::ToolStderrDelta { event_id, .. }
            | RuntimeEvent::ToolExit { event_id, .. }
            | RuntimeEvent::ToolResultOk { event_id, .. }
            | RuntimeEvent::ToolResultErr { event_id, .. }
            | RuntimeEvent::InputInjected { event_id, .. }
            | RuntimeEvent::RetryTimerFired { event_id, .. }
            | RuntimeEvent::TransientError { event_id, .. }
            | RuntimeEvent::FatalError { event_id, .. }
            | RuntimeEvent::CancelRequested { event_id, .. } => event_id,
        }
    }

    pub fn with_new_id(self) -> Self {
        let eid = new_id();
        match self {
            RuntimeEvent::TurnStarted { turn_id, input, .. } => RuntimeEvent::TurnStarted {
                event_id: eid,
                turn_id,
                input,
            },
            RuntimeEvent::ModelTextDelta { epoch, delta, .. } => RuntimeEvent::ModelTextDelta {
                event_id: eid,
                epoch,
                delta,
            },
            RuntimeEvent::ModelReasoningDelta { epoch, delta, .. } => {
                RuntimeEvent::ModelReasoningDelta {
                    event_id: eid,
                    epoch,
                    delta,
                }
            }
            RuntimeEvent::ModelToolCall { epoch, call, .. } => RuntimeEvent::ModelToolCall {
                event_id: eid,
                epoch,
                call,
            },
            RuntimeEvent::ModelCompleted { epoch, usage, .. } => RuntimeEvent::ModelCompleted {
                event_id: eid,
                epoch,
                usage,
            },
            RuntimeEvent::ToolDispatched { epoch, call_id, .. } => RuntimeEvent::ToolDispatched {
                event_id: eid,
                epoch,
                call_id,
            },
            RuntimeEvent::ToolStdoutDelta {
                epoch,
                call_id,
                delta,
                ..
            } => RuntimeEvent::ToolStdoutDelta {
                event_id: eid,
                epoch,
                call_id,
                delta,
            },
            RuntimeEvent::ToolStderrDelta {
                epoch,
                call_id,
                delta,
                ..
            } => RuntimeEvent::ToolStderrDelta {
                event_id: eid,
                epoch,
                call_id,
                delta,
            },
            RuntimeEvent::ToolExit {
                epoch,
                call_id,
                exit_code,
                duration_ms,
                ..
            } => RuntimeEvent::ToolExit {
                event_id: eid,
                epoch,
                call_id,
                exit_code,
                duration_ms,
            },
            RuntimeEvent::ToolResultOk { epoch, result, .. } => RuntimeEvent::ToolResultOk {
                event_id: eid,
                epoch,
                result,
            },
            RuntimeEvent::ToolResultErr { epoch, result, .. } => RuntimeEvent::ToolResultErr {
                event_id: eid,
                epoch,
                result,
            },
            RuntimeEvent::InputInjected { input, .. } => RuntimeEvent::InputInjected {
                event_id: eid,
                input,
            },
            RuntimeEvent::RetryTimerFired { next_epoch, .. } => RuntimeEvent::RetryTimerFired {
                event_id: eid,
                next_epoch,
            },
            RuntimeEvent::TransientError {
                epoch,
                message,
                retry_after_ms,
                ..
            } => RuntimeEvent::TransientError {
                event_id: eid,
                epoch,
                message,
                retry_after_ms,
            },
            RuntimeEvent::FatalError { message, .. } => RuntimeEvent::FatalError {
                event_id: eid,
                message,
            },
            RuntimeEvent::CancelRequested { reason, .. } => RuntimeEvent::CancelRequested {
                event_id: eid,
                reason,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::RuntimeEvent;

    #[test]
    fn runtime_event_contract_no_longer_contains_redundant_tool_queue_variants() {
        let queued = json!({
            "type": "tool_queued",
            "event_id": "e1",
            "epoch": 0,
            "call_id": "c1",
            "tool_name": "echo"
        });
        let dequeued = json!({
            "type": "tool_dequeued",
            "event_id": "e2",
            "epoch": 0,
            "call_id": "c1",
            "tool_name": "echo"
        });

        assert!(
            serde_json::from_value::<RuntimeEvent>(queued).is_err(),
            "tool_queued runtime event should not deserialize anymore"
        );
        assert!(
            serde_json::from_value::<RuntimeEvent>(dequeued).is_err(),
            "tool_dequeued runtime event should not deserialize anymore"
        );
    }
}
