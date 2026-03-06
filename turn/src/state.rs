use argus_core::ToolCall;

use crate::{TurnContext, TurnFailure, TurnSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLlmStep {
    pub step_index: u32,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBatch {
    pub step_index: u32,
    pub calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnState {
    Ready(TurnContext),
    StreamingLlm(ActiveLlmStep),
    WaitingTools(ToolBatch),
    Completed(TurnSummary),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
