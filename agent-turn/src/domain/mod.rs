#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    Noop { id: String },
}

