use crate::command::DomainCommand;
use crate::domain::DomainEvent;
use crate::handlers::CommandHandler;
use crate::state::TurnState;

pub struct LegacyHandler;

impl CommandHandler for LegacyHandler {
    fn handle(&self, cmd: &DomainCommand, _state: &TurnState) -> Vec<DomainEvent> {
        match cmd {
            DomainCommand::RuntimeEvent(event) => {
                vec![DomainEvent::LegacyRuntimeEvent {
                    event: event.clone(),
                }]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use agent_core::{RuntimeEvent, SessionMeta};

    use crate::command::DomainCommand;
    use crate::domain::DomainEvent;
    use crate::handlers::HandlerRegistry;
    use crate::state::TurnState;

    #[test]
    fn runtime_event_command_is_forwarded_to_legacy_runtime_domain_event() {
        let reg = HandlerRegistry::default();
        let cmd = DomainCommand::RuntimeEvent(RuntimeEvent::FatalError {
            event_id: "e1".into(),
            message: "boom".into(),
        });
        let out = reg.handle(cmd, &TurnState::new(SessionMeta::new("s", "t"), "p", "m"));
        assert!(matches!(
            out.as_slice(),
            [DomainEvent::LegacyRuntimeEvent {
                event: RuntimeEvent::FatalError { .. }
            }]
        ));
    }
}

