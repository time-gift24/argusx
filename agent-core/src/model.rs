use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::transcript_item::TranscriptItem;

pub type Id = String;

pub fn new_id() -> Id {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSource {
    User,
    Tool,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputPart {
    Text { text: String },
    Json { value: serde_json::Value },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputEnvelope {
    pub id: Id,
    pub source: InputSource,
    pub parts: Vec<InputPart>,
}

impl InputEnvelope {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            source: InputSource::User,
            parts: vec![InputPart::Text { text: text.into() }],
        }
    }

    pub fn tool_json(value: serde_json::Value) -> Self {
        Self {
            id: new_id(),
            source: InputSource::Tool,
            parts: vec![InputPart::Json { value }],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: Id,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

impl ToolCall {
    pub fn new(tool_name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            call_id: new_id(),
            tool_name: tool_name.into(),
            arguments,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: Id,
    pub output: serde_json::Value,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(call_id: impl Into<String>, output: serde_json::Value) -> Self {
        Self {
            call_id: call_id.into(),
            output,
            is_error: false,
        }
    }

    pub fn err(call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            output: serde_json::json!({"error": message.into()}),
            is_error: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl Usage {
    pub fn merge(&mut self, other: &Usage) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.total_tokens = self.total_tokens.saturating_add(other.total_tokens);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: Id,
    pub turn_id: Id,
}

impl SessionMeta {
    pub fn new(session_id: impl Into<String>, turn_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            turn_id: turn_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelOutputEvent {
    TextDelta { delta: String },
    ReasoningDelta { delta: String },
    ToolCall { call: ToolCall },
    Completed { usage: Option<Usage> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub epoch: u64,
    pub transcript: Vec<TranscriptItem>,
    pub inputs: Vec<InputEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRequest {
    pub meta: SessionMeta,
    pub initial_input: InputEnvelope,
    #[serde(default)]
    pub transcript: Vec<TranscriptItem>,
}

impl TurnRequest {
    pub fn new(meta: SessionMeta, initial_input: InputEnvelope) -> Self {
        Self {
            meta,
            initial_input,
            transcript: Vec::new(),
        }
    }
}
