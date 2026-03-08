use std::sync::Arc;

use crate::{TurnFailure, TurnFinishReason, TurnMessageSnapshot, TurnSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTurn {
    pub turn_id: String,
    pub transcript: TurnMessageSnapshot,
    pub assistant_text: Option<Arc<str>>,
    pub finish_reason: TurnFinishReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnOutcome {
    Completed(CompletedTurn),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
