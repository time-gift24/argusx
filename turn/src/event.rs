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
    Failed { message: String, retryable: bool },
    TimedOut,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnFinishReason {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurnEvent {
    TurnStarted,
    LlmTextDelta {
        text: String,
    },
    LlmReasoningDelta {
        text: String,
    },
    ToolCallPrepared {
        call: ToolCall,
    },
    ToolCallCompleted {
        call_id: String,
        result: ToolOutcome,
    },
    ToolCallPermissionRequested {
        request: PermissionRequest,
    },
    ToolCallPermissionResolved {
        request_id: String,
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
