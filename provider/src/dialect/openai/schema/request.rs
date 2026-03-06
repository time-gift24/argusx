use crate::dialect::openai::schema::common::{
    ReasoningEffort, ResponseFormat, Role, StopSequences, Tool, ToolCall, ToolChoice, Verbosity,
    map_is_empty,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ChatCompletionsOptions {
    pub model: String,
    pub messages: Vec<ChatMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<std::collections::HashMap<String, i32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "max_tokens")]
    pub max_completion_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequences>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<Verbosity>,

    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

impl ChatCompletionsOptions {
    pub fn apply_stream_defaults(&mut self) {
        if self.stream == Some(true) {
            let stream_options = self
                .stream_options
                .get_or_insert_with(StreamOptions::default);
            if stream_options.include_usage.is_none() {
                stream_options.include_usage = Some(true);
            }
        }
    }

    pub fn normalized_for_send(mut self) -> Self {
        self.apply_stream_defaults();
        self
    }

    pub fn to_legacy_json(&self) -> Result<Value, serde_json::Error> {
        let mut normalized = self.clone();
        normalized.apply_stream_defaults();
        let mut value = serde_json::to_value(normalized)?;
        if let Some(obj) = value.as_object_mut()
            && let Some(v) = obj.remove("max_completion_tokens")
        {
            obj.insert("max_tokens".to_string(), v);
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: Role,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,

    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}
