use crate::dialect::util::map_is_empty;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZaiStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ZaiChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ZaiUsage>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZaiUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZaiChoice {
    pub index: u32,
    #[serde(default)]
    pub delta: ZaiDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ZaiAssistantMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ZaiDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ZaiDeltaToolCall>>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZaiAssistantMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ZaiDeltaToolCall>>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZaiDeltaToolCall {
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ZaiDeltaFunction>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZaiDeltaFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZaiStreamEvent {
    Chunk(ZaiStreamChunk),
    Done,
    Error(String),
    Open,
}
