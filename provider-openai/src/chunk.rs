use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionsChunk {
    pub id: String,
    pub created: i64,
    #[serde(rename = "object")]
    pub object_type: String,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(rename = "usage")]
    pub usage: Option<ChunkUsage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChunkUsage {
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u64,
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u64,
    #[serde(rename = "total_tokens")]
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Choice {
    pub index: u32,
    #[serde(rename = "delta")]
    pub delta: Delta,
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Delta {
    #[serde(rename = "content")]
    pub content: Option<String>,
    #[serde(rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
    #[serde(rename = "tool_calls")]
    pub tool_calls: Option<Vec<ToolCallChunk>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCallChunk {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    pub index: Option<u32>,
    pub function: FunctionChunk,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionChunk {
    pub name: Option<String>,
    pub arguments: Option<String>,
}
