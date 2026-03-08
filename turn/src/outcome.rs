use crate::{TurnFinishReason, TurnMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    pub turn_id: String,
    pub finish_reason: TurnFinishReason,
    pub transcript: Vec<TurnMessage>,
    pub final_output: Option<String>,
}
