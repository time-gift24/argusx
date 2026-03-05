use crate::command::DomainCommand;
use crate::domain::DomainEvent;
use crate::handlers::CommandHandler;
use crate::state::TurnState;

pub struct InputHandler;

impl CommandHandler for InputHandler {
    fn handle(&self, cmd: &DomainCommand, _state: &TurnState) -> Vec<DomainEvent> {
        match cmd {
            DomainCommand::InputInjected { input, .. } => {
                vec![DomainEvent::InputQueued {
                    input_id: input.id.clone(),
                }]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use agent_core::{InputEnvelope, SessionMeta};

    use crate::command::DomainCommand;
    use crate::domain::DomainEvent;
    use crate::handlers::HandlerRegistry;
    use crate::state::TurnState;

    #[test]
    fn input_injected_command_emits_input_queued_event() {
        let reg = HandlerRegistry::default();
        let cmd = DomainCommand::InputInjected {
            id: "c1".into(),
            input: InputEnvelope::user_text("hello"),
        };
        let out = reg.handle(cmd, &TurnState::new(SessionMeta::new("s", "t"), "p", "m"));
        assert!(matches!(out.as_slice(), [DomainEvent::InputQueued { .. }]));
    }
}

