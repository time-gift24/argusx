use crate::command::DomainCommand;
use crate::domain::DomainEvent;
use crate::state::TurnState;

pub mod input;
pub mod legacy;
pub mod lifecycle;
pub mod model;
pub mod subagent;
pub mod tool;

pub trait CommandHandler {
    fn handle(&self, cmd: &DomainCommand, state: &TurnState) -> Vec<DomainEvent>;
}

pub struct HandlerRegistry {
    handlers: Vec<Box<dyn CommandHandler + Send + Sync>>,
}

impl HandlerRegistry {
    pub fn new(handlers: Vec<Box<dyn CommandHandler + Send + Sync>>) -> Self {
        Self { handlers }
    }

    pub fn handle(&self, cmd: DomainCommand, state: &TurnState) -> Vec<DomainEvent> {
        let mut out = Vec::new();
        for handler in &self.handlers {
            out.extend(handler.handle(&cmd, state));
        }
        out
    }
}

impl HandlerRegistry {
    pub fn with_defaults() -> Self {
        Self::new(vec![
            Box::new(model::ModelHandler),
            Box::new(input::InputHandler),
            Box::new(tool::ToolHandler),
            Box::new(lifecycle::LifecycleHandler),
            Box::new(subagent::SubagentHandler),
            Box::new(legacy::LegacyHandler),
        ])
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
