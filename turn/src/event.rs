use std::sync::Arc;

use argus_core::ToolCall;
use serde_json::Value;

use crate::{PermissionDecision, PermissionRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepFinishReason {
    ToolCalls,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    Success(Value),
    Failed { message: Arc<str>, retryable: bool },
    TimedOut,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnFinishReason {
    Completed,
    Cancelled,
    Failed,
    /// The turn reached `max_steps` and `FinalStepPolicy::Fail` was set, or
    /// the forced-text final step still returned tool calls.
    MaxStepsExceeded,
    /// The model returned `FinishReason::Length` (output was truncated).
    ModelLengthLimit,
    /// The model returned an unrecognised finish reason.
    ModelProtocolError,
    /// A model-start or stream-idle timeout fired.
    LlmTimeout,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurnEvent {
    TurnStarted,
    LlmTextDelta {
        text: Arc<str>,
    },
    LlmReasoningDelta {
        text: Arc<str>,
    },
    ToolCallPrepared {
        call: Arc<ToolCall>,
    },
    ToolCallCompleted {
        call_id: Arc<str>,
        result: ToolOutcome,
    },
    ToolCallPermissionRequested {
        request: PermissionRequest,
    },
    ToolCallPermissionResolved {
        request_id: Arc<str>,
        decision: PermissionDecision,
    },
    StepFinished {
        step_index: u32,
        reason: StepFinishReason,
    },
    TurnFinished {
        reason: TurnFinishReason,
    },
}
