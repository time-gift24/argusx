use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use turn::{TurnError, TurnMessage, TurnMessageSnapshot};

use super::events::TurnTargetKind;

pub trait ConversationRepository: Send + Sync {
    fn save(&self, record: ConversationRecord) -> Result<(), TurnError>;
    fn load(&self, conversation_id: &str) -> Result<Option<ConversationRecord>, TurnError>;
    fn list(&self) -> Result<Vec<ConversationRecord>, TurnError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationRecord {
    pub conversation_id: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
    pub history: Vec<StoredConversationMessage>,
    pub updated_at_ms: u64,
}

impl ConversationRecord {
    pub fn from_snapshot(
        conversation_id: impl Into<String>,
        target_kind: TurnTargetKind,
        target_id: impl Into<String>,
        history: TurnMessageSnapshot,
        updated_at_ms: u64,
    ) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            target_kind,
            target_id: target_id.into(),
            history: history
                .iter()
                .map(|message| StoredConversationMessage::from_turn_message(message.as_ref()))
                .collect(),
            updated_at_ms,
        }
    }

    pub fn to_snapshot(&self) -> TurnMessageSnapshot {
        Arc::from(
            self.history
                .iter()
                .cloned()
                .map(|message| Arc::new(message.into_turn_message()))
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StoredConversationMessage {
    User {
        content: String,
    },
    AssistantText {
        content: String,
    },
    AssistantToolCalls {
        content: Option<String>,
        calls: Vec<StoredToolCall>,
    },
    ToolResult {
        call_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
    SystemNote {
        content: String,
    },
}

impl StoredConversationMessage {
    pub(crate) fn from_turn_message(message: &TurnMessage) -> Self {
        match message {
            TurnMessage::User { content } => Self::User {
                content: content.to_string(),
            },
            TurnMessage::AssistantText { content } => Self::AssistantText {
                content: content.to_string(),
            },
            TurnMessage::AssistantToolCalls { content, calls } => Self::AssistantToolCalls {
                content: content.as_ref().map(ToString::to_string),
                calls: calls
                    .iter()
                    .map(|call| StoredToolCall::from_tool_call(call.as_ref()))
                    .collect(),
            },
            TurnMessage::ToolResult {
                call_id,
                tool_name,
                content,
                is_error,
            } => Self::ToolResult {
                call_id: call_id.to_string(),
                tool_name: tool_name.to_string(),
                content: content.to_string(),
                is_error: *is_error,
            },
            TurnMessage::SystemNote { content } => Self::SystemNote {
                content: content.to_string(),
            },
        }
    }

    pub(crate) fn into_turn_message(self) -> TurnMessage {
        match self {
            Self::User { content } => TurnMessage::User {
                content: content.into(),
            },
            Self::AssistantText { content } => TurnMessage::AssistantText {
                content: content.into(),
            },
            Self::AssistantToolCalls { content, calls } => TurnMessage::AssistantToolCalls {
                content: content.map(Into::into),
                calls: Arc::from(
                    calls.into_iter()
                        .map(|call| Arc::new(call.into_tool_call()))
                        .collect::<Vec<_>>(),
                ),
            },
            Self::ToolResult {
                call_id,
                tool_name,
                content,
                is_error,
            } => TurnMessage::ToolResult {
                call_id: call_id.into(),
                tool_name: tool_name.into(),
                content: content.into(),
                is_error,
            },
            Self::SystemNote { content } => TurnMessage::SystemNote {
                content: content.into(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StoredToolCall {
    FunctionCall {
        sequence: u32,
        call_id: String,
        name: String,
        arguments_json: String,
    },
    Builtin {
        sequence: u32,
        call_id: String,
        builtin_name: String,
        arguments_json: String,
    },
    Mcp {
        sequence: u32,
        id: String,
        mcp_type: String,
        server_label: Option<String>,
        name: Option<String>,
        arguments_json: Option<String>,
        output_json: Option<String>,
        tools_json: Option<String>,
        error: Option<String>,
    },
}

impl StoredToolCall {
    fn from_tool_call(call: &argus_core::ToolCall) -> Self {
        match call {
            argus_core::ToolCall::FunctionCall {
                sequence,
                call_id,
                name,
                arguments_json,
            } => Self::FunctionCall {
                sequence: *sequence,
                call_id: call_id.clone(),
                name: name.clone(),
                arguments_json: arguments_json.clone(),
            },
            argus_core::ToolCall::Builtin(call) => Self::Builtin {
                sequence: call.sequence,
                call_id: call.call_id.clone(),
                builtin_name: call.builtin.canonical_name().to_string(),
                arguments_json: call.arguments_json.clone(),
            },
            argus_core::ToolCall::Mcp(call) => Self::Mcp {
                sequence: call.sequence,
                id: call.id.clone(),
                mcp_type: match &call.mcp_type {
                    argus_core::McpCallType::McpListTools => "mcp_list_tools".to_string(),
                    argus_core::McpCallType::McpCall => "mcp_call".to_string(),
                    argus_core::McpCallType::Unknown(value) => value.clone(),
                },
                server_label: call.server_label.clone(),
                name: call.name.clone(),
                arguments_json: call.arguments_json.clone(),
                output_json: call.output_json.clone(),
                tools_json: call.tools_json.clone(),
                error: call.error.clone(),
            },
        }
    }

    fn into_tool_call(self) -> argus_core::ToolCall {
        match self {
            Self::FunctionCall {
                sequence,
                call_id,
                name,
                arguments_json,
            } => argus_core::ToolCall::FunctionCall {
                sequence,
                call_id,
                name,
                arguments_json,
            },
            Self::Builtin {
                sequence,
                call_id,
                builtin_name,
                arguments_json,
            } => argus_core::ToolCall::Builtin(argus_core::BuiltinToolCall {
                sequence,
                call_id,
                builtin: argus_core::Builtin::from_name(&builtin_name)
                    .unwrap_or(argus_core::Builtin::Unknown(builtin_name)),
                arguments_json,
            }),
            Self::Mcp {
                sequence,
                id,
                mcp_type,
                server_label,
                name,
                arguments_json,
                output_json,
                tools_json,
                error,
            } => argus_core::ToolCall::Mcp(argus_core::McpCall {
                sequence,
                id,
                mcp_type: match mcp_type.as_str() {
                    "mcp_list_tools" => argus_core::McpCallType::McpListTools,
                    "mcp_call" => argus_core::McpCallType::McpCall,
                    other => argus_core::McpCallType::Unknown(other.to_string()),
                },
                server_label,
                name,
                arguments_json,
                output_json,
                tools_json,
                error,
            }),
        }
    }
}

#[derive(Default)]
pub struct InMemoryConversationRepository {
    records: Mutex<HashMap<String, ConversationRecord>>,
}

impl ConversationRepository for InMemoryConversationRepository {
    fn save(&self, record: ConversationRecord) -> Result<(), TurnError> {
        self.records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation repository lock poisoned".to_string()))?
            .insert(record.conversation_id.clone(), record);
        Ok(())
    }

    fn load(&self, conversation_id: &str) -> Result<Option<ConversationRecord>, TurnError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation repository lock poisoned".to_string()))?
            .get(conversation_id)
            .cloned())
    }

    fn list(&self) -> Result<Vec<ConversationRecord>, TurnError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation repository lock poisoned".to_string()))?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sort_records(&mut records);
        Ok(records)
    }
}

pub struct JsonConversationRepository {
    path: PathBuf,
    records: Mutex<HashMap<String, ConversationRecord>>,
}

impl JsonConversationRepository {
    pub fn new(path: PathBuf) -> Result<Self, TurnError> {
        let records = load_records(&path)?;

        Ok(Self {
            path,
            records: Mutex::new(records),
        })
    }

    fn flush(&self, records: &HashMap<String, ConversationRecord>) -> Result<(), TurnError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                TurnError::Runtime(format!("create conversation store directory: {error}"))
            })?;
        }

        let mut listed = records.values().cloned().collect::<Vec<_>>();
        sort_records(&mut listed);
        let payload = serde_json::to_string_pretty(&listed)
            .map_err(|error| TurnError::Runtime(format!("serialize conversations: {error}")))?;

        fs::write(&self.path, payload)
            .map_err(|error| TurnError::Runtime(format!("write conversations: {error}")))
    }
}

impl ConversationRepository for JsonConversationRepository {
    fn save(&self, record: ConversationRecord) -> Result<(), TurnError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation repository lock poisoned".to_string()))?;
        records.insert(record.conversation_id.clone(), record);
        self.flush(&records)
    }

    fn load(&self, conversation_id: &str) -> Result<Option<ConversationRecord>, TurnError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation repository lock poisoned".to_string()))?
            .get(conversation_id)
            .cloned())
    }

    fn list(&self) -> Result<Vec<ConversationRecord>, TurnError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation repository lock poisoned".to_string()))?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sort_records(&mut records);
        Ok(records)
    }
}

fn load_records(path: &PathBuf) -> Result<HashMap<String, ConversationRecord>, TurnError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|error| TurnError::Runtime(format!("read conversations: {error}")))?;
    let listed: Vec<ConversationRecord> = serde_json::from_str(&raw)
        .map_err(|error| TurnError::Runtime(format!("parse conversations: {error}")))?;

    Ok(listed
        .into_iter()
        .map(|record| (record.conversation_id.clone(), record))
        .collect())
}

fn sort_records(records: &mut [ConversationRecord]) {
    records.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| left.conversation_id.cmp(&right.conversation_id))
    });
}
