use crate::command::DomainCommand;
use crate::domain::DomainEvent;
use crate::handlers::CommandHandler;
use crate::state::TurnState;

pub struct SubagentHandler;

impl CommandHandler for SubagentHandler {
    fn handle(&self, _cmd: &DomainCommand, _state: &TurnState) -> Vec<DomainEvent> {
        Vec::new()
    }
}
