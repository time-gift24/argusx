use crate::domain::DomainEvent;
use crate::state::TurnState;

pub struct StateProjector;

impl StateProjector {
    pub fn apply(state: &mut TurnState, event: &DomainEvent) {
        if let DomainEvent::ModelChunkArrived { delta, .. } = event {
            state.output_buffer.push_str(delta);
        }
    }
}

#[cfg(test)]
mod tests {
    use agent_core::SessionMeta;

    use crate::domain::DomainEvent;
    use crate::projectors::state::StateProjector;
    use crate::state::TurnState;

    #[test]
    fn state_projector_updates_output_buffer_on_model_chunk() {
        let mut s = TurnState::new(SessionMeta::new("s1", "t1"), "p", "m");
        StateProjector::apply(
            &mut s,
            &DomainEvent::ModelChunkArrived {
                epoch: 0,
                delta: "hello".into(),
            },
        );
        assert_eq!(s.output_buffer, "hello");
    }
}

