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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubAgentToolSnapshot {
    pub call_id: Id,
    pub tool_name: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunStreamEvent {
    TurnStart {
        turn_id: Id,
    },
    ReasoningStarted {
        turn_id: Id,
    },
    ReasoningCompleted {
        turn_id: Id,
        truncated: bool,
        char_count: u32,
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
    ToolQueued {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolDequeued {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolExecutionStart {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolStdoutDelta {
        turn_id: Id,
        call_id: Id,
        delta: String,
    },
    ToolStderrDelta {
        turn_id: Id,
        call_id: Id,
        delta: String,
    },
    ToolExit {
        turn_id: Id,
        call_id: Id,
        exit_code: Option<i32>,
        duration_ms: u64,
    },
    ToolExecutionDone {
        turn_id: Id,
        result: ToolResult,
    },
    ToolExecutionError {
        turn_id: Id,
        result: ToolResult,
    },
    SubAgentUpdated {
        turn_id: Id,
        thread_id: Id,
        agent_name: String,
        status: String,
        active_tools: Vec<SubAgentToolSnapshot>,
        error: Option<String>,
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
        arguments: serde_json::Value,
    },
    ReasoningStarted {
        turn_id: Id,
    },
    ReasoningCompleted {
        turn_id: Id,
        truncated: bool,
        char_count: u32,
    },
    ToolQueued {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolDequeued {
        turn_id: Id,
        call_id: Id,
        tool_name: String,
    },
    ToolCallProgress {
        turn_id: Id,
        call_id: Id,
        status: ToolCallStatus,
    },
    ToolStdoutDelta {
        turn_id: Id,
        call_id: Id,
        delta: String,
    },
    ToolStderrDelta {
        turn_id: Id,
        call_id: Id,
        delta: String,
    },
    ToolExit {
        turn_id: Id,
        call_id: Id,
        exit_code: Option<i32>,
        duration_ms: u64,
    },
    ToolCallCompleted {
        turn_id: Id,
        result: ToolResult,
    },
    SubAgentUpdated {
        turn_id: Id,
        thread_id: Id,
        agent_name: String,
        status: String,
        active_tools: Vec<SubAgentToolSnapshot>,
        error: Option<String>,
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
