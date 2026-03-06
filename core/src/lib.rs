use std::sync::Arc;

pub struct Meta {
    pub model: String,
    pub provider: String,
}

pub enum ToolCall {
    FunctionCall {
        call_id: String,
        name: String,
        arguments_json: String,
    },
    Mcp {
        server: String,
        method: String,
        payload_json: String,
        call_id: Option<String>,
    },
}

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
        Error { message: message.to_string() }
    }
}

#[derive(Debug)]
pub struct ResponseContract {
    terminated: bool,
}

impl ResponseContract {
    pub fn new() -> Self {
        Self { terminated: false }
    }

    pub fn accept(&mut self, event: ResponseEvent) -> Result<(), ContractError> {
        if self.terminated {
            return Err(ContractError::AfterTerminal);
        }
        if matches!(event, ResponseEvent::Done(_) | ResponseEvent::Error(_)) {
            self.terminated = true;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ContractError {
    AfterTerminal,
}

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
