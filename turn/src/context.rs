#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnContext {
    pub session_id: String,
    pub user_message: String,
}
