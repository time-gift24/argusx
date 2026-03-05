use crate::command::DomainCommand;
use crate::domain::DomainEvent;
use crate::handlers::CommandHandler;
use crate::state::TurnState;

pub struct ModelHandler;

impl CommandHandler for ModelHandler {
    fn handle(&self, cmd: &DomainCommand, _state: &TurnState) -> Vec<DomainEvent> {
        match cmd {
            DomainCommand::ModelTextDelta { epoch, delta, .. } => {
                vec![DomainEvent::ModelChunkArrived {
                    epoch: *epoch,
                    delta: delta.clone(),
                }]
            }
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
    fn model_text_delta_command_emits_model_chunk_arrived_event() {
        let reg = HandlerRegistry::default();
        let cmd = DomainCommand::ModelTextDelta {
            id: "c1".into(),
            epoch: 0,
            delta: "hello".into(),
        };
        let out = reg.handle(cmd, &TurnState::new(SessionMeta::new("s", "t"), "p", "m"));
        assert!(matches!(
            out.as_slice(),
            [DomainEvent::ModelChunkArrived { .. }]
        ));
    }
}
