use agent_core::RuntimeEvent;

pub mod normalizer;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainCommand {
    Noop { id: String },
    RuntimeEvent(RuntimeEvent),
}

impl DomainCommand {
    pub fn id(&self) -> &str {
        match self {
            Self::Noop { id } => id,
            Self::RuntimeEvent(ev) => ev.id(),
        }
    }

    pub fn from_runtime(event: RuntimeEvent) -> Self {
        Self::RuntimeEvent(event)
    }
}
