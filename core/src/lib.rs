use futures::Stream;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{sync::mpsc, task::AbortHandle};

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
    Builtin(BuiltinToolCall),
    Mcp(McpCall),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinToolCall {
    pub sequence: u32,
    pub call_id: String,
    pub builtin: Builtin,
    pub arguments_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Builtin {
    Read,
    Glob,
    Grep,
    UpdatePlan,
    Shell,
    DomainCookies,
    Git,
    Unknown(String),
}

impl Builtin {
    pub fn canonical_name(&self) -> &str {
        match self {
            Self::Read => "read",
            Self::Glob => "glob",
            Self::Grep => "grep",
            Self::UpdatePlan => "update_plan",
            Self::Shell => "shell",
            Self::DomainCookies => "domain_cookies",
            Self::Git => "git",
            Self::Unknown(name) => name.as_str(),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "read" => Self::Read,
            "glob" => Self::Glob,
            "grep" => Self::Grep,
            "update_plan" => Self::UpdatePlan,
            "shell" => Self::Shell,
            "domain_cookies" => Self::DomainCookies,
            "git" => Self::Git,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCall {
    pub sequence: u32,
    pub id: String,
    pub mcp_type: McpCallType,
    pub server_label: Option<String>,
    pub name: Option<String>,
    pub arguments_json: Option<String>,
    pub output_json: Option<String>,
    pub tools_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpCallType {
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
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    Cancelled,
    Unknown(String),
}

impl FinishReason {
    pub fn from_wire(reason: &str) -> Self {
        match reason {
            "stop" => Self::Stop,
            "tool_calls" => Self::ToolCalls,
            "length" => Self::Length,
            "cancelled" => Self::Cancelled,
            other => Self::Unknown(other.to_string()),
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
        if matches!(event, ResponseEvent::Done { .. } | ResponseEvent::Error(_)) {
            self.terminated = true;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    AfterTerminal,
}

pub struct ResponseStream {
    rx_event: mpsc::Receiver<ResponseEvent>,
    abort: Option<AbortHandle>,
}

impl ResponseStream {
    pub fn from_parts(rx_event: mpsc::Receiver<ResponseEvent>, abort: AbortHandle) -> Self {
        Self {
            rx_event,
            abort: Some(abort),
        }
    }
}

impl Stream for ResponseStream {
    type Item = ResponseEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx_event.poll_recv(cx)
    }
}

impl Drop for ResponseStream {
    fn drop(&mut self) {
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
    }
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
    Done {
        reason: FinishReason,
        usage: Option<Usage>,
    },
    Error(Error),
}
