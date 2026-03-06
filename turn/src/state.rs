use argus_core::ToolCall;

use crate::{PermissionRequest, TurnContext, TurnFailure, TurnSummary};

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
pub struct PendingPermissionCall {
    pub request: PermissionRequest,
    pub call: ToolCall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPause {
    pub batch: ToolBatch,
    pub pending: Vec<PendingPermissionCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnState {
    Ready(TurnContext),
    StreamingLlm(ActiveLlmStep),
    WaitingTools(ToolBatch),
    WaitingForPermission(PermissionPause),
    Completed(TurnSummary),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
