use serde::{Deserialize, Serialize};

use crate::model::{Id, ToolResult, Usage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Planned,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnStats {
    pub tool_calls_count: u32,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunStreamEvent {
    TurnStart {
        turn_id: Id,
    },
    InputInjected {
        turn_id: Id,
        input_id: Id,
    },
    ToolExecutionPlanned {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolExecutionStart {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolExecutionDone {
        turn_id: Id,
        result: ToolResult,
    },
    ToolExecutionError {
        turn_id: Id,
        result: ToolResult,
    },
    ModelCompleted {
        turn_id: Id,
        usage: Option<Usage>,
    },
    Retrying {
        turn_id: Id,
        attempt: u32,
        next_epoch: u64,
        delay_ms: u64,
    },
    TransientError {
        turn_id: Id,
        message: String,
        can_retry: bool,
    },
    ProtocolWarning {
        turn_id: Id,
        message: String,
    },
    TurnDone {
        turn_id: Id,
        epoch: u64,
        final_message: Option<String>,
        usage: Usage,
        stats: TurnStats,
    },
    TurnFailed {
        turn_id: Id,
        epoch: u64,
        message: String,
        usage: Usage,
        stats: TurnStats,
        cancelled: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiThreadEvent {
    MessageDelta {
        turn_id: Id,
        delta: String,
    },
    ReasoningDelta {
        turn_id: Id,
        delta: String,
    },
    ToolCallRequested {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolCallProgress {
        turn_id: Id,
        call_id: Id,
        status: ToolCallStatus,
    },
    ToolCallCompleted {
        turn_id: Id,
        result: ToolResult,
    },
    Warning {
        turn_id: Id,
        message: String,
    },
    Error {
        turn_id: Id,
        message: String,
    },
    Done {
        turn_id: Id,
        summary: Option<String>,
        stats: TurnStats,
    },
}
