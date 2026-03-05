use crate::command::DomainCommand;
use crate::domain::DomainEvent;
use crate::handlers::CommandHandler;
use crate::state::TurnState;

pub struct LifecycleHandler;

impl CommandHandler for LifecycleHandler {
    fn handle(&self, cmd: &DomainCommand, _state: &TurnState) -> Vec<DomainEvent> {
        match cmd {
            DomainCommand::RetryTimerFired { next_epoch, .. } => vec![DomainEvent::RetryFired {
                next_epoch: *next_epoch,
            }],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use agent_core::SessionMeta;

    use crate::command::DomainCommand;
    use crate::domain::DomainEvent;
    use crate::handlers::HandlerRegistry;
    use crate::state::TurnState;

    #[test]
    fn retry_timer_fired_emits_retry_fired_event() {
        let reg = HandlerRegistry::default();
        let cmd = DomainCommand::RetryTimerFired {
            id: "c1".into(),
            next_epoch: 3,
        };
        let out = reg.handle(cmd, &TurnState::new(SessionMeta::new("s", "t"), "p", "m"));
        assert!(matches!(out.as_slice(), [DomainEvent::RetryFired { .. }]));
    }
}
