use crate::{
    PermissionRequest, TurnFailure, TurnSummary,
    transcript::{SharedToolCall, SharedToolCalls},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLlmStep {
    pub step_index: u32,
    pub tool_calls: Vec<SharedToolCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBatch {
    pub step_index: u32,
    pub calls: SharedToolCalls,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPermissionCall {
    pub request: PermissionRequest,
    pub call_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPause {
    pub batch: ToolBatch,
    pub pending: Vec<PendingPermissionCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnState {
    Ready,
    StreamingLlm(ActiveLlmStep),
    WaitingTools(ToolBatch),
    WaitingForPermission(PermissionPause),
    Completed(TurnSummary),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
