#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnFinishReason {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEvent {
    TurnStarted,
    TurnFinished { reason: TurnFinishReason },
}
