use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use turn::TurnError;

use super::events::TurnTargetKind;

pub trait ConversationThreadRepository: Send + Sync {
    fn save_thread(&self, record: ConversationThreadRecord) -> Result<(), TurnError>;
    fn load_thread(&self, conversation_id: &str) -> Result<Option<ConversationThreadRecord>, TurnError>;
    fn list_threads(&self) -> Result<Vec<ConversationThreadRecord>, TurnError>;
    fn active_thread_id(&self) -> Result<Option<String>, TurnError>;
    fn set_active_thread(&self, conversation_id: Option<String>) -> Result<(), TurnError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Idle,
    Running,
    Restartable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationThreadRecord {
    pub conversation_id: String,
    pub title: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
    pub updated_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_prompt: Option<String>,
    pub status: ThreadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationThreadSummary {
    pub conversation_id: String,
    pub title: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
    pub updated_at_ms: u64,
    pub status: ThreadStatus,
    pub is_active: bool,
}

impl ConversationThreadSummary {
    pub fn from_record(record: ConversationThreadRecord, is_active: bool) -> Self {
        Self {
            conversation_id: record.conversation_id,
            title: record.title,
            target_kind: record.target_kind,
            target_id: record.target_id,
            updated_at_ms: record.updated_at_ms,
            status: record.status,
            is_active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationThreadInput {
    pub title: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SwitchConversationThreadInput {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestartConversationInput {
    pub conversation_id: String,
}

#[derive(Default)]
pub struct InMemoryConversationThreadRepository {
    active_thread_id: Mutex<Option<String>>,
    records: Mutex<HashMap<String, ConversationThreadRecord>>,
}

impl ConversationThreadRepository for InMemoryConversationThreadRepository {
    fn save_thread(&self, record: ConversationThreadRecord) -> Result<(), TurnError> {
        self.records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))?
            .insert(record.conversation_id.clone(), record);
        Ok(())
    }

    fn load_thread(&self, conversation_id: &str) -> Result<Option<ConversationThreadRecord>, TurnError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))?
            .get(conversation_id)
            .cloned())
    }

    fn list_threads(&self) -> Result<Vec<ConversationThreadRecord>, TurnError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sort_threads(&mut records);
        Ok(records)
    }

    fn active_thread_id(&self) -> Result<Option<String>, TurnError> {
        self.active_thread_id
            .lock()
            .map(|value| value.clone())
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))
    }

    fn set_active_thread(&self, conversation_id: Option<String>) -> Result<(), TurnError> {
        *self
            .active_thread_id
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))? =
            conversation_id;
        Ok(())
    }
}

pub struct JsonConversationThreadRepository {
    path: PathBuf,
    state: Mutex<ThreadCatalogFile>,
}

impl JsonConversationThreadRepository {
    pub fn new(path: PathBuf) -> Result<Self, TurnError> {
        Ok(Self {
            state: Mutex::new(load_catalog(&path)?),
            path,
        })
    }

    fn flush(&self, state: &ThreadCatalogFile) -> Result<(), TurnError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                TurnError::Runtime(format!("create conversation thread directory: {error}"))
            })?;
        }

        let mut serialized = state.clone();
        sort_threads(&mut serialized.threads);
        let payload = serde_json::to_string_pretty(&serialized)
            .map_err(|error| TurnError::Runtime(format!("serialize conversation threads: {error}")))?;

        fs::write(&self.path, payload)
            .map_err(|error| TurnError::Runtime(format!("write conversation threads: {error}")))
    }
}

impl ConversationThreadRepository for JsonConversationThreadRepository {
    fn save_thread(&self, record: ConversationThreadRecord) -> Result<(), TurnError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))?;
        if let Some(existing) = state
            .threads
            .iter_mut()
            .find(|thread| thread.conversation_id == record.conversation_id)
        {
            *existing = record;
        } else {
            state.threads.push(record);
        }
        self.flush(&state)
    }

    fn load_thread(&self, conversation_id: &str) -> Result<Option<ConversationThreadRecord>, TurnError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))?
            .threads
            .iter()
            .find(|thread| thread.conversation_id == conversation_id)
            .cloned())
    }

    fn list_threads(&self) -> Result<Vec<ConversationThreadRecord>, TurnError> {
        let mut records = self
            .state
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))?
            .threads
            .clone();
        sort_threads(&mut records);
        Ok(records)
    }

    fn active_thread_id(&self) -> Result<Option<String>, TurnError> {
        self.state
            .lock()
            .map(|state| state.active_conversation_id.clone())
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))
    }

    fn set_active_thread(&self, conversation_id: Option<String>) -> Result<(), TurnError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| TurnError::Runtime("conversation thread repository lock poisoned".to_string()))?;
        state.active_conversation_id = conversation_id;
        self.flush(&state)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ThreadCatalogFile {
    active_conversation_id: Option<String>,
    threads: Vec<ConversationThreadRecord>,
}

fn load_catalog(path: &PathBuf) -> Result<ThreadCatalogFile, TurnError> {
    if !path.exists() {
        return Ok(ThreadCatalogFile::default());
    }

    let raw = fs::read_to_string(path)
        .map_err(|error| TurnError::Runtime(format!("read conversation threads: {error}")))?;
    serde_json::from_str(&raw)
        .map_err(|error| TurnError::Runtime(format!("parse conversation threads: {error}")))
}

fn sort_threads(threads: &mut [ConversationThreadRecord]) {
    threads.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| left.conversation_id.cmp(&right.conversation_id))
    });
}
