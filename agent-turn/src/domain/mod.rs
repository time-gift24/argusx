use agent_core::RuntimeEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainEvent {
    Noop {
        id: String,
    },
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
    LegacyRuntimeEvent {
        event: RuntimeEvent,
    },
}
