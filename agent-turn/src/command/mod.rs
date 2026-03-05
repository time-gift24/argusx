use agent_core::{InputEnvelope, RuntimeEvent};

pub mod normalizer;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainCommand {
    Noop { id: String },
    ModelTextDelta {
        id: String,
        epoch: u64,
        delta: String,
    },
    InputInjected {
        id: String,
        input: InputEnvelope,
    },
    RuntimeEvent(RuntimeEvent),
}

impl DomainCommand {
    pub fn id(&self) -> &str {
        match self {
            Self::Noop { id } => id,
            Self::ModelTextDelta { id, .. } => id,
            Self::InputInjected { id, .. } => id,
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
            RuntimeEvent::InputInjected { event_id, input } => Self::InputInjected {
                id: event_id,
                input,
            },
            other => Self::RuntimeEvent(other),
        }
    }
}
