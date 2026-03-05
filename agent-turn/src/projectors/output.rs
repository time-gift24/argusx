use agent_core::UiThreadEvent;

use crate::domain::DomainEvent;
use crate::output::OutputEvent;
use crate::state::TurnState;

pub struct OutputProjector;

impl OutputProjector {
    pub fn map(state: &TurnState, event: &DomainEvent) -> Vec<OutputEvent> {
        match event {
            DomainEvent::ModelChunkArrived { delta, .. } => {
                vec![OutputEvent::Ui(UiThreadEvent::MessageDelta {
                    turn_id: state.meta.turn_id.clone(),
                    delta: delta.clone(),
                })]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use agent_core::SessionMeta;

    use crate::domain::DomainEvent;
    use crate::output::OutputEvent;
    use crate::projectors::output::OutputProjector;
    use crate::state::TurnState;

    #[test]
    fn output_projector_maps_model_chunk_to_ui_message_delta() {
        let state = TurnState::new(SessionMeta::new("s1", "t1"), "p", "m");
        let out = OutputProjector::map(
            &state,
            &DomainEvent::ModelChunkArrived {
                epoch: 0,
                delta: "hello".into(),
            },
        );
        assert!(matches!(
            out.as_slice(),
            [OutputEvent::Ui(agent_core::UiThreadEvent::MessageDelta { .. })]
        ));
    }
}

