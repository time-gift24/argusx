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
}
