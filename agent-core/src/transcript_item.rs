use serde::{Deserialize, Serialize};

use crate::model::{new_id, Id, InputEnvelope, ToolCall, ToolResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscriptItem {
    UserMessage {
        id: Id,
        input: InputEnvelope,
    },
    AssistantMessage {
        id: Id,
        text: String,
    },
    Reasoning {
        id: Id,
        text: String,
        #[serde(default)]
        truncated: bool,
        #[serde(default)]
        char_count: u32,
    },
    ToolCall {
        id: Id,
        epoch: u64,
        call: ToolCall,
    },
    ToolResult {
        id: Id,
        epoch: u64,
        result: ToolResult,
    },
    SystemNote {
        id: Id,
        level: NoteLevel,
        message: String,
    },
}

impl TranscriptItem {
    pub fn user_message(input: InputEnvelope) -> Self {
        Self::UserMessage {
            id: new_id(),
            input,
        }
    }

    pub fn assistant_message(text: impl Into<String>) -> Self {
        Self::AssistantMessage {
            id: new_id(),
            text: text.into(),
        }
    }

    pub fn reasoning(text: impl Into<String>) -> Self {
        let text = text.into();
        let char_count = text.chars().count() as u32;
        Self::Reasoning {
            id: new_id(),
            text,
            truncated: false,
            char_count,
        }
    }

    pub fn reasoning_with_meta(
        text: impl Into<String>,
        truncated: bool,
        char_count: u32,
    ) -> Self {
        Self::Reasoning {
            id: new_id(),
            text: text.into(),
            truncated,
            char_count,
        }
    }

    pub fn tool_call(epoch: u64, call: ToolCall) -> Self {
        Self::ToolCall {
            id: new_id(),
            epoch,
            call,
        }
    }

    pub fn tool_result(epoch: u64, result: ToolResult) -> Self {
        Self::ToolResult {
            id: new_id(),
            epoch,
            result,
        }
    }

    pub fn system_note(level: NoteLevel, message: impl Into<String>) -> Self {
        Self::SystemNote {
            id: new_id(),
            level,
            message: message.into(),
        }
    }
}
