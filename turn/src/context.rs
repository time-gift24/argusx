use crate::TurnMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSeed {
    pub session_id: String,
    pub turn_id: String,
    pub prior_messages: Vec<TurnMessage>,
    pub user_message: String,
    pub system_prompt: Option<String>,
}
