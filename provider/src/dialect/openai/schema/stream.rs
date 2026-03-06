use crate::dialect::openai::schema::common::{LogProbs, Role, StreamError, map_is_empty};
use crate::dialect::openai::schema::response::Usage;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionsStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamChoice {
    pub index: u32,
    #[serde(default)]
    pub delta: Delta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogProbs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<DeltaToolCall>>,

    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeltaToolCall {
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<DeltaFunction>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeltaFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatCompletionsStreamEvent {
    Chunk(ChatCompletionsStreamChunk),
    Done,
    Error(StreamError),
    Open,
}
