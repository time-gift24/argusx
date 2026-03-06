use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub id: String,
    pub created: i64,
    pub object: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    FunctionCall {
        sequence: u32,
        call_id: String,
        name: String,
        arguments_json: String,
    },
    Mcp(ZaiMcpCall),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZaiMcpCall {
    pub sequence: u32,
    pub id: String,
    pub mcp_type: ZaiMcpType,
    pub server_label: Option<String>,
    pub name: Option<String>,
    pub arguments_json: Option<String>,
    pub output_json: Option<String>,
    pub tools_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZaiMcpType {
    McpListTools,
    McpCall,
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl Usage {
    pub fn zero() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Error { message }
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Error {
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ResponseContract {
    terminated: bool,
}

impl ResponseContract {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept(&mut self, event: &ResponseEvent) -> Result<(), ContractError> {
        if self.terminated {
            return Err(ContractError::AfterTerminal);
        }
        if matches!(event, ResponseEvent::Done(_) | ResponseEvent::Error(_)) {
            self.terminated = true;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    AfterTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseEvent {
    Created(Meta),
    ContentDelta(Arc<str>),
    ReasoningDelta(Arc<str>),
    ToolDelta(Arc<str>),
    ContentDone(String),
    ReasoningDone(String),
    ToolDone(ToolCall),
    Done(Option<Usage>),
    Error(Error),
}
