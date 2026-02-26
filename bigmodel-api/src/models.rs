use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Multimodal(Vec<ContentPart>),
    Json(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
    VideoUrl { video_url: UrlResource },
    FileUrl { file_url: UrlResource },
    InputAudio { input_audio: InputAudio },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlResource {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudio {
    pub data: String,
    pub format: AudioFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Wav,
    Mp3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Tool {
    #[serde(rename = "function")]
    Function(FunctionTool),
    #[serde(rename = "retrieval")]
    Retrieval(RetrievalTool),
    #[serde(rename = "web_search")]
    WebSearch(WebSearchTool),
    #[serde(rename = "mcp")]
    MCP(McpTool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionTool {
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalTool {
    pub retrieval: RetrievalObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalObject {
    pub knowledge_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchTool {
    pub web_search: WebSearchObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub mcp: McpObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpObject {
    pub server_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thinking {
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_thinking: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub type_field: ResponseFormatType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormatType {
    Text,
    JsonObject,
}

// Response types

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub web_search: Vec<WebSearchResult>,
    #[serde(default)]
    pub video_result: Vec<VideoResult>,
    #[serde(default)]
    pub content_filter: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    #[serde(rename = "finish_reason")]
    pub finish_reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: i32,
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: i32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebSearchResult {
    pub icon: Option<String>,
    pub title: Option<String>,
    pub link: Option<String>,
    pub media: Option<String>,
    pub publish_date: Option<String>,
    pub content: Option<String>,
    pub refer: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoResult {
    pub url: Option<String>,
    pub cover_image_url: Option<String>,
}

// Streaming response

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponseChunk {
    pub id: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChoiceChunk>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChoiceChunk {
    pub index: i32,
    #[serde(rename = "delta")]
    pub delta: Delta,
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeltaToolCall {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub function: Option<DeltaToolFunction>,
    pub index: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeltaToolFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

// Builder methods

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            do_sample: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: false,
            tool_stream: None,
            tools: None,
            tool_choice: None,
            stop: None,
            response_format: None,
            request_id: None,
            user_id: None,
            thinking: None,
        }
    }

    pub fn do_sample(mut self, value: bool) -> Self {
        self.do_sample = Some(value);
        self
    }

    pub fn temperature(mut self, value: f32) -> Self {
        self.temperature = Some(value);
        self
    }

    pub fn max_tokens(mut self, value: i32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    pub fn stream(mut self) -> Self {
        self.stream = true;
        self
    }

    pub fn tool_stream(mut self, value: bool) -> Self {
        self.tool_stream = Some(value);
        self
    }

    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn top_p(mut self, value: f32) -> Self {
        self.top_p = Some(value);
        self
    }

    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn response_format(mut self, response_format: ResponseFormat) -> Self {
        self.response_format = Some(response_format);
        self
    }

    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn thinking(mut self, thinking: Thinking) -> Self {
        self.thinking = Some(thinking);
        self
    }
}

impl Message {
    pub fn user(content: impl Into<Content>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            reasoning_content: None,
        }
    }

    pub fn assistant(content: impl Into<Content>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            reasoning_content: None,
        }
    }

    pub fn system(content: impl Into<Content>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            reasoning_content: None,
        }
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}
