use agent_core::{InputEnvelope, RuntimeEvent};

pub mod normalizer;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainCommand {
    Noop {
        id: String,
    },
    ModelTextDelta {
        id: String,
        epoch: u64,
        delta: String,
    },
    InputInjected {
        id: String,
        input: InputEnvelope,
    },
    ToolResultOk {
        id: String,
        epoch: u64,
        result: agent_core::ToolResult,
    },
    ToolResultErr {
        id: String,
        epoch: u64,
        result: agent_core::ToolResult,
    },
    RetryTimerFired {
        id: String,
        next_epoch: u64,
    },
    RuntimeEvent(RuntimeEvent),
}

impl DomainCommand {
    pub fn id(&self) -> &str {
        match self {
            Self::Noop { id } => id,
            Self::ModelTextDelta { id, .. } => id,
            Self::InputInjected { id, .. } => id,
            Self::ToolResultOk { id, .. } => id,
            Self::ToolResultErr { id, .. } => id,
            Self::RetryTimerFired { id, .. } => id,
            Self::RuntimeEvent(ev) => ev.id(),
        }
    }

    pub fn from_runtime(event: RuntimeEvent) -> Self {
        match event {
            RuntimeEvent::ModelTextDelta {
                event_id,
                epoch,
                delta,
            } => Self::ModelTextDelta {
                id: event_id,
                epoch,
                delta,
            },
            other => Self::RuntimeEvent(other),
        }
    }
}
