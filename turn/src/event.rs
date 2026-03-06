#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnFinishReason {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEvent {
    TurnStarted,
    LlmTextDelta { text: String },
    LlmReasoningDelta { text: String },
    TurnFinished { reason: TurnFinishReason },
}
