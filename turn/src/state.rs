use crate::{TurnContext, TurnFailure, TurnSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnState {
    Ready(TurnContext),
    Completed(TurnSummary),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
