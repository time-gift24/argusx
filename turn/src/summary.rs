#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSummary {
    pub turn_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFailure {
    pub message: String,
}
