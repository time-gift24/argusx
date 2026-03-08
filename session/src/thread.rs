use uuid::Uuid;

#[derive(Debug)]
pub struct ThreadRuntime {
    pub thread_id: Uuid,
    pub active_turn: Option<ActiveTurnRuntime>,
}

impl ThreadRuntime {
    pub fn new(thread_id: Uuid) -> Self {
        Self {
            thread_id,
            active_turn: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveTurnRuntime {
    pub turn_id: Uuid,
    pub turn_number: u32,
}
