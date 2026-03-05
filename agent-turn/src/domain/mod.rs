#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    Noop { id: String },
    ModelChunkArrived {
        epoch: u64,
        delta: String,
    },
    InputQueued {
        input_id: String,
    },
    ToolFinished {
        epoch: u64,
        call_id: String,
        is_error: bool,
    },
    RetryFired {
        next_epoch: u64,
    },
}
