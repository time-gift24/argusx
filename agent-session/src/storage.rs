use std::collections::HashMap;
use std::sync::Arc;

use agent_core::{
    AgentError, CheckpointStore, SessionInfo, SessionStatus, TranscriptItem, TurnContext,
    TurnSummary,
};
use anyhow::Result;
use async_trait::async_trait;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone)]
pub struct SessionFilter {
    pub user_id: Option<String>,
    pub status: Option<SessionStatus>,
    pub from_date: Option<i64>,
    pub to_date: Option<i64>,
    pub limit: Option<usize>,
}

impl Default for SessionFilter {
    fn default() -> Self {
        Self {
            user_id: None,
            status: None,
            from_date: None,
            to_date: None,
            limit: Some(100),
        }
    }
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, info: &SessionInfo) -> Result<()>;
    async fn get(&self, session_id: &str) -> Result<Option<SessionInfo>>;
    async fn update(&self, info: &SessionInfo) -> Result<()>;
    async fn delete(&self, session_id: &str) -> Result<()>;
    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;
}

pub struct FileSessionStore {
    base_path: PathBuf,
}

impl FileSessionStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn checked_session_path(&self, session_id: &str) -> Result<PathBuf> {
        validate_session_id(session_id)?;
        Ok(self.base_path.join(session_id))
    }

    fn checked_metadata_path(&self, session_id: &str) -> Result<PathBuf> {
        Ok(self.checked_session_path(session_id)?.join("metadata.json"))
    }

    fn checked_turns_path(&self, session_id: &str) -> Result<PathBuf> {
        Ok(self.checked_session_path(session_id)?.join("turns"))
    }

    fn checked_turn_path(&self, session_id: &str, turn_id: &str) -> Result<PathBuf> {
        validate_turn_id(turn_id)?;
        Ok(self.checked_turns_path(session_id)?.join(turn_id))
    }

    fn checked_turn_context_path(&self, session_id: &str, turn_id: &str) -> Result<PathBuf> {
        Ok(self
            .checked_turn_path(session_id, turn_id)?
            .join("context.json"))
    }

    fn checked_turn_summary_path(&self, session_id: &str, turn_id: &str) -> Result<PathBuf> {
        Ok(self
            .checked_turn_path(session_id, turn_id)?
            .join("summary.json"))
    }

    fn checked_turn_transcript_path(&self, session_id: &str, turn_id: &str) -> Result<PathBuf> {
        Ok(self
            .checked_turn_path(session_id, turn_id)?
            .join("transcript.jsonl"))
    }

    pub async fn save_turn_context(&self, context: &TurnContext) -> Result<()> {
        let turn_path = self.checked_turn_path(&context.session_id, &context.turn_id)?;
        fs::create_dir_all(&turn_path).await?;
        let context_path = self.checked_turn_context_path(&context.session_id, &context.turn_id)?;
        let raw = serde_json::to_string_pretty(context)?;
        fs::write(context_path, raw).await?;
        Ok(())
    }

    pub async fn save_turn_summary(&self, session_id: &str, summary: &TurnSummary) -> Result<()> {
        let turn_path = self.checked_turn_path(session_id, &summary.turn_id)?;
        fs::create_dir_all(&turn_path).await?;
        let summary_path = self.checked_turn_summary_path(session_id, &summary.turn_id)?;
        let raw = serde_json::to_string_pretty(summary)?;
        fs::write(summary_path, raw).await?;
        Ok(())
    }

    pub async fn list_turn_summaries(&self, session_id: &str) -> Result<Vec<TurnSummary>> {
        let mut summaries = Vec::new();
        let turns_path = self.checked_turns_path(session_id)?;
        if !fs::try_exists(&turns_path).await? {
            return Ok(summaries);
        }

        let mut entries = fs::read_dir(turns_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let summary_path = path.join("summary.json");
            if !fs::try_exists(&summary_path).await? {
                continue;
            }

            if let Ok(raw) = fs::read_to_string(&summary_path).await {
                if let Ok(summary) = serde_json::from_str::<TurnSummary>(&raw) {
                    summaries.push(summary);
                }
            }
        }

        summaries.sort_by_key(|summary| summary.started_at);
        Ok(summaries)
    }

    pub async fn load_latest_turn_summary(&self, session_id: &str) -> Result<Option<TurnSummary>> {
        let mut summaries = self.list_turn_summaries(session_id).await?;
        Ok(summaries.pop())
    }

    pub async fn save_turn_transcript(
        &self,
        session_id: &str,
        turn_id: &str,
        items: &[TranscriptItem],
    ) -> Result<()> {
        let turn_path = self.checked_turn_path(session_id, turn_id)?;
        fs::create_dir_all(&turn_path).await?;
        let transcript_path = self.checked_turn_transcript_path(session_id, turn_id)?;

        let mut lines = Vec::with_capacity(items.len());
        for item in items {
            lines.push(serde_json::to_string(item)?);
        }
        let payload = if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        };

        fs::write(transcript_path, payload).await?;
        Ok(())
    }

    pub async fn load_turn_transcript(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Vec<TranscriptItem>> {
        let transcript_path = self.checked_turn_transcript_path(session_id, turn_id)?;
        if !fs::try_exists(&transcript_path).await? {
            return Ok(Vec::new());
        }

        let raw = fs::read_to_string(transcript_path).await?;
        let mut items = Vec::new();
        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            let item = serde_json::from_str::<TranscriptItem>(line)?;
            items.push(item);
        }
        Ok(items)
    }

    pub async fn find_session_id_by_turn_id(&self, turn_id: &str) -> Result<Option<String>> {
        validate_turn_id(turn_id)?;
        if !fs::try_exists(&self.base_path).await? {
            return Ok(None);
        }

        let mut entries = fs::read_dir(&self.base_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };

            let candidate = path.join("turns").join(turn_id);
            if fs::try_exists(&candidate).await? {
                return Ok(Some(name.to_string()));
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl SessionStore for FileSessionStore {
    async fn create(&self, info: &SessionInfo) -> Result<()> {
        let path = self.checked_session_path(&info.session_id)?;
        fs::create_dir_all(&path).await?;

        let metadata_path = self.checked_metadata_path(&info.session_id)?;
        let json = serde_json::to_string_pretty(info)?;
        fs::write(metadata_path, json).await?;

        info!("Created session: {}", info.session_id);
        Ok(())
    }

    async fn get(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        let metadata_path = self.checked_metadata_path(session_id)?;

        if !fs::try_exists(&metadata_path).await? {
            return Ok(None);
        }

        let content = fs::read_to_string(metadata_path).await?;
        let info: SessionInfo = serde_json::from_str(&content)?;
        Ok(Some(info))
    }

    async fn update(&self, info: &SessionInfo) -> Result<()> {
        let metadata_path = self.checked_metadata_path(&info.session_id)?;

        if !fs::try_exists(&metadata_path).await? {
            anyhow::bail!("Session not found: {}", info.session_id);
        }

        let json = serde_json::to_string_pretty(info)?;
        fs::write(metadata_path, json).await?;
        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let path = self.checked_session_path(session_id)?;

        if fs::try_exists(&path).await? {
            fs::remove_dir_all(&path).await?;
            info!("Deleted session: {}", session_id);
        }
        Ok(())
    }

    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        let mut sessions = Vec::new();

        if !fs::try_exists(&self.base_path).await? {
            return Ok(sessions);
        }

        let mut entries = fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if fs::try_exists(&metadata_path).await? {
                    if let Ok(content) = fs::read_to_string(metadata_path).await {
                        if let Ok(info) = serde_json::from_str::<SessionInfo>(&content) {
                            // Apply filters
                            if let Some(ref user_id) = filter.user_id {
                                if info.user_id.as_deref() != Some(user_id.as_str()) {
                                    continue;
                                }
                            }
                            if let Some(ref status) = filter.status {
                                if info.status != *status {
                                    continue;
                                }
                            }
                            if let Some(from_date) = filter.from_date {
                                if info.updated_at < from_date {
                                    continue;
                                }
                            }
                            if let Some(to_date) = filter.to_date {
                                if info.updated_at > to_date {
                                    continue;
                                }
                            }
                            sessions.push(info);
                        }
                    }
                }
            }
        }

        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        if let Some(limit) = filter.limit {
            sessions.truncate(limit);
        }

        Ok(sessions)
    }
}

fn validate_session_id(session_id: &str) -> Result<()> {
    validate_id("session", session_id)
}

fn validate_turn_id(turn_id: &str) -> Result<()> {
    validate_id("turn", turn_id)
}

fn validate_id(kind: &str, id: &str) -> Result<()> {
    if id.is_empty() {
        anyhow::bail!("{kind} id cannot be empty");
    }

    let mut components = Path::new(id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => anyhow::bail!("invalid {kind} id: {id}"),
    }
}

pub struct FileTurnCheckpointStore {
    store: Arc<FileSessionStore>,
    turn_to_session: Arc<RwLock<HashMap<String, String>>>,
}

impl FileTurnCheckpointStore {
    pub fn new(store: Arc<FileSessionStore>) -> Self {
        Self {
            store,
            turn_to_session: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_turn(&self, session_id: &str, turn_id: &str) -> Result<()> {
        validate_session_id(session_id)?;
        validate_turn_id(turn_id)?;
        let mut guard = self.turn_to_session.write().await;
        guard.insert(turn_id.to_string(), session_id.to_string());
        Ok(())
    }

    async fn resolve_session_id(&self, turn_id: &str) -> Result<Option<String>> {
        {
            let guard = self.turn_to_session.read().await;
            if let Some(session_id) = guard.get(turn_id) {
                return Ok(Some(session_id.clone()));
            }
        }

        self.store.find_session_id_by_turn_id(turn_id).await
    }
}

#[async_trait]
impl CheckpointStore for FileTurnCheckpointStore {
    async fn append_items(
        &self,
        turn_id: &str,
        items: &[TranscriptItem],
    ) -> Result<(), AgentError> {
        let Some(session_id) = self
            .resolve_session_id(turn_id)
            .await
            .map_err(to_agent_error)?
        else {
            return Err(AgentError::Internal {
                message: format!("turn not found for append_items: {turn_id}"),
            });
        };

        let mut existing = self
            .store
            .load_turn_transcript(&session_id, turn_id)
            .await
            .map_err(to_agent_error)?;
        existing.extend_from_slice(items);
        self.store
            .save_turn_transcript(&session_id, turn_id, &existing)
            .await
            .map_err(to_agent_error)
    }

    async fn load_items(&self, turn_id: &str) -> Result<Vec<TranscriptItem>, AgentError> {
        let Some(session_id) = self
            .resolve_session_id(turn_id)
            .await
            .map_err(to_agent_error)?
        else {
            return Ok(Vec::new());
        };

        self.store
            .load_turn_transcript(&session_id, turn_id)
            .await
            .map_err(to_agent_error)
    }

    async fn snapshot(&self, turn_id: &str, items: &[TranscriptItem]) -> Result<(), AgentError> {
        let Some(session_id) = self
            .resolve_session_id(turn_id)
            .await
            .map_err(to_agent_error)?
        else {
            return Err(AgentError::Internal {
                message: format!("turn not found for snapshot: {turn_id}"),
            });
        };

        self.store
            .save_turn_transcript(&session_id, turn_id, items)
            .await
            .map_err(to_agent_error)
    }
}

fn to_agent_error(error: anyhow::Error) -> AgentError {
    AgentError::Internal {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::{InputEnvelope, SessionMeta, TurnStatus};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_get_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let info = SessionInfo::new("s1".into(), "Test Session".into());
        store.create(&info).await.unwrap();

        let retrieved = store.get("s1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Session");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        for i in 1..=3 {
            let info = SessionInfo::new(format!("s{}", i), format!("Session {}", i));
            store.create(&info).await.unwrap();
        }

        let sessions = store.list(SessionFilter::default()).await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_filter_sessions_by_user_id() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let mut s1 = SessionInfo::new("s1".into(), "Session 1".into());
        s1.user_id = Some("user-a".into());
        store.create(&s1).await.expect("create session 1");

        let mut s2 = SessionInfo::new("s2".into(), "Session 2".into());
        s2.user_id = Some("user-b".into());
        store.create(&s2).await.expect("create session 2");

        let sessions = store
            .list(SessionFilter {
                user_id: Some("user-a".into()),
                ..SessionFilter::default()
            })
            .await
            .expect("list sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "s1");
    }

    #[tokio::test]
    async fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let info = SessionInfo::new("s1".into(), "Test".into());
        store.create(&info).await.unwrap();

        store.delete("s1").await.unwrap();

        let retrieved = store.get("s1").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_rejects_path_traversal_session_id() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let err = store.get("../escape").await.expect_err("should reject");
        assert!(err.to_string().contains("invalid session id"));
    }

    #[tokio::test]
    async fn test_turn_artifacts_roundtrip() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());
        let session = SessionInfo::new("s1".into(), "session".into());
        store.create(&session).await.expect("create session");

        let context = TurnContext {
            turn_id: "t1".into(),
            session_id: "s1".into(),
            epoch: 0,
            started_at: 100,
        };
        store
            .save_turn_context(&context)
            .await
            .expect("save turn context");

        let summary = TurnSummary {
            turn_id: "t1".into(),
            epoch: 0,
            started_at: 100,
            ended_at: Some(200),
            status: TurnStatus::Done,
            final_message: Some("ok".into()),
            tool_calls_count: 1,
            input_tokens: 10,
            output_tokens: 20,
        };
        store
            .save_turn_summary("s1", &summary)
            .await
            .expect("save turn summary");

        let transcript = vec![
            TranscriptItem::user_message(InputEnvelope::user_text("hello")),
            TranscriptItem::assistant_message("world"),
        ];
        store
            .save_turn_transcript("s1", "t1", &transcript)
            .await
            .expect("save transcript");

        let loaded_summaries = store
            .list_turn_summaries("s1")
            .await
            .expect("list summaries");
        assert_eq!(loaded_summaries.len(), 1);
        assert_eq!(loaded_summaries[0].turn_id, "t1");

        let latest = store
            .load_latest_turn_summary("s1")
            .await
            .expect("latest summary")
            .expect("summary exists");
        assert_eq!(latest.turn_id, "t1");

        let loaded_transcript = store
            .load_turn_transcript("s1", "t1")
            .await
            .expect("load transcript");
        assert_eq!(loaded_transcript.len(), 2);

        let checkpoint_store = FileTurnCheckpointStore::new(Arc::new(store));
        checkpoint_store
            .register_turn("s1", "t1")
            .await
            .expect("register turn");
        let loaded = checkpoint_store
            .load_items("t1")
            .await
            .expect("load from checkpoint store");
        assert_eq!(loaded.len(), 2);

        // Silence unused import lint for SessionMeta in test scope and ensure ID helpers stay serializable.
        let _ = SessionMeta::new("s1", "t1");
    }
}
