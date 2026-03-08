use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use turn::{TurnError, TurnMessageSnapshot};

use super::{
    events::TurnTargetKind,
    storage::StoredConversationMessage,
};

pub trait ConversationCheckpointRepository: Send + Sync {
    fn save(&self, record: ConversationCheckpointRecord) -> Result<(), TurnError>;
    fn load(&self, checkpoint_id: &str) -> Result<Option<ConversationCheckpointRecord>, TurnError>;
    fn list(&self) -> Result<Vec<ConversationCheckpointRecord>, TurnError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationCheckpointRecord {
    pub checkpoint_id: String,
    pub source_conversation_id: String,
    pub title: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
    pub created_at_ms: u64,
    pub history: Vec<StoredConversationMessage>,
}

impl ConversationCheckpointRecord {
    pub fn from_snapshot(
        checkpoint_id: impl Into<String>,
        source_conversation_id: impl Into<String>,
        title: impl Into<String>,
        target_kind: TurnTargetKind,
        target_id: impl Into<String>,
        history: TurnMessageSnapshot,
        created_at_ms: u64,
    ) -> Self {
        Self {
            checkpoint_id: checkpoint_id.into(),
            source_conversation_id: source_conversation_id.into(),
            title: title.into(),
            target_kind,
            target_id: target_id.into(),
            created_at_ms,
            history: history
                .iter()
                .map(|message| StoredConversationMessage::from_turn_message(message.as_ref()))
                .collect(),
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
pub struct ConversationCheckpointSummary {
    pub checkpoint_id: String,
    pub source_conversation_id: String,
    pub title: String,
    pub created_at_ms: u64,
}

impl From<&ConversationCheckpointRecord> for ConversationCheckpointSummary {
    fn from(value: &ConversationCheckpointRecord) -> Self {
        Self {
            checkpoint_id: value.checkpoint_id.clone(),
            source_conversation_id: value.source_conversation_id.clone(),
            title: value.title.clone(),
            created_at_ms: value.created_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationCheckpointInput {
    pub conversation_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreConversationCheckpointInput {
    pub checkpoint_id: String,
    pub title: Option<String>,
}

#[derive(Default)]
pub struct InMemoryConversationCheckpointRepository {
    records: Mutex<HashMap<String, ConversationCheckpointRecord>>,
}

impl ConversationCheckpointRepository for InMemoryConversationCheckpointRepository {
    fn save(&self, record: ConversationCheckpointRecord) -> Result<(), TurnError> {
        self.records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation checkpoint repository lock poisoned".to_string()))?
            .insert(record.checkpoint_id.clone(), record);
        Ok(())
    }

    fn load(&self, checkpoint_id: &str) -> Result<Option<ConversationCheckpointRecord>, TurnError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation checkpoint repository lock poisoned".to_string()))?
            .get(checkpoint_id)
            .cloned())
    }

    fn list(&self) -> Result<Vec<ConversationCheckpointRecord>, TurnError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation checkpoint repository lock poisoned".to_string()))?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sort_records(&mut records);
        Ok(records)
    }
}

pub struct JsonConversationCheckpointRepository {
    path: PathBuf,
    records: Mutex<HashMap<String, ConversationCheckpointRecord>>,
}

impl JsonConversationCheckpointRepository {
    pub fn new(path: PathBuf) -> Result<Self, TurnError> {
        let records = load_records(&path)?;

        Ok(Self {
            path,
            records: Mutex::new(records),
        })
    }

    fn flush(&self, records: &HashMap<String, ConversationCheckpointRecord>) -> Result<(), TurnError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                TurnError::Runtime(format!("create checkpoint store directory: {error}"))
            })?;
        }

        let mut listed = records.values().cloned().collect::<Vec<_>>();
        sort_records(&mut listed);
        let payload = serde_json::to_string_pretty(&listed)
            .map_err(|error| TurnError::Runtime(format!("serialize checkpoints: {error}")))?;

        fs::write(&self.path, payload)
            .map_err(|error| TurnError::Runtime(format!("write checkpoints: {error}")))
    }
}

impl ConversationCheckpointRepository for JsonConversationCheckpointRepository {
    fn save(&self, record: ConversationCheckpointRecord) -> Result<(), TurnError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation checkpoint repository lock poisoned".to_string()))?;
        records.insert(record.checkpoint_id.clone(), record);
        self.flush(&records)
    }

    fn load(&self, checkpoint_id: &str) -> Result<Option<ConversationCheckpointRecord>, TurnError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation checkpoint repository lock poisoned".to_string()))?
            .get(checkpoint_id)
            .cloned())
    }

    fn list(&self) -> Result<Vec<ConversationCheckpointRecord>, TurnError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| TurnError::Runtime("conversation checkpoint repository lock poisoned".to_string()))?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sort_records(&mut records);
        Ok(records)
    }
}

fn load_records(path: &PathBuf) -> Result<HashMap<String, ConversationCheckpointRecord>, TurnError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|error| TurnError::Runtime(format!("read checkpoints: {error}")))?;
    let listed: Vec<ConversationCheckpointRecord> = serde_json::from_str(&raw)
        .map_err(|error| TurnError::Runtime(format!("parse checkpoints: {error}")))?;

    Ok(listed
        .into_iter()
        .map(|record| (record.checkpoint_id.clone(), record))
        .collect())
}

fn sort_records(records: &mut [ConversationCheckpointRecord]) {
    records.sort_by(|left, right| {
        right
            .created_at_ms
            .cmp(&left.created_at_ms)
            .then_with(|| left.checkpoint_id.cmp(&right.checkpoint_id))
    });
}
