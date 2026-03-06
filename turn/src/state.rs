use crate::{TurnContext, TurnFailure, TurnSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLlmStep {
    pub step_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnState {
    Ready(TurnContext),
    StreamingLlm(ActiveLlmStep),
    Completed(TurnSummary),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
