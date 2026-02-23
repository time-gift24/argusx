use agent_core::{RunStreamEvent, UiThreadEvent};
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatTurnStatus {
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatResponse {
    pub turn_id: String,
    pub status: ChatTurnStatus,
    pub final_message: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStreamEvent {
    Run(RunStreamEvent),
    Ui(UiThreadEvent),
}

pub type AgentStream = Pin<Box<dyn Stream<Item = AgentStreamEvent> + Send>>;
