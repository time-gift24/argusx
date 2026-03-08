use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const TURN_EVENT_NAME: &str = "turn-event";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TurnTargetKind {
    Agent,
    Workflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartConversationInput {
    pub prompt: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContinueConversationInput {
    pub conversation_id: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelConversationInput {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopTurnEvent {
    pub conversation_id: String,
    pub turn_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: Value,
}

impl DesktopTurnEvent {
    pub fn new(
        conversation_id: impl Into<String>,
        turn_id: impl Into<String>,
        event_type: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            turn_id: turn_id.into(),
            event_type: event_type.into(),
            data,
        }
    }

    pub fn text_delta(
        conversation_id: impl Into<String>,
        turn_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self::new(
            conversation_id,
            turn_id,
            "llm-text-delta",
            json!({ "text": text.into() }),
        )
    }
}
