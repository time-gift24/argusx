use agent_core::{SessionId, SessionInfo, SessionStatus};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct SessionFilter {
    pub user_id: Option<String>,
    pub status: Option<SessionStatus>,
    pub limit: Option<usize>,
}

impl Default for SessionFilter {
    fn default() -> Self {
        Self {
            user_id: None,
            status: None,
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

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.base_path.join(session_id)
    }

    fn metadata_path(&self, session_id: &str) -> PathBuf {
        self.session_path(session_id).join("metadata.json")
    }
}

#[async_trait]
impl SessionStore for FileSessionStore {
    async fn create(&self, info: &SessionInfo) -> Result<()> {
        let path = self.session_path(&info.session_id);
        fs::create_dir_all(&path).await?;

        let metadata_path = self.metadata_path(&info.session_id);
        let json = serde_json::to_string_pretty(info)?;
        fs::write(metadata_path, json).await?;

        info!("Created session: {}", info.session_id);
        Ok(())
    }

    async fn get(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        let metadata_path = self.metadata_path(&session_id.to_string());

        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(metadata_path).await?;
        let info: SessionInfo = serde_json::from_str(&content)?;
        Ok(Some(info))
    }

    async fn update(&self, info: &SessionInfo) -> Result<()> {
        let metadata_path = self.metadata_path(&info.session_id);

        if !metadata_path.exists() {
            anyhow::bail!("Session not found: {}", info.session_id);
        }

        let json = serde_json::to_string_pretty(info)?;
        fs::write(metadata_path, json).await?;
        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let path = self.session_path(&session_id.to_string());

        if path.exists() {
            fs::remove_dir_all(path).await?;
            info!("Deleted session: {}", session_id);
        }
        Ok(())
    }

    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        let mut sessions = Vec::new();

        let mut entries = fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(content) = fs::read_to_string(metadata_path).await {
                        if let Ok(info) = serde_json::from_str::<SessionInfo>(&content) {
                            // Apply filters
                            if let Some(ref status) = filter.status {
                                if info.status != *status {
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

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let info = SessionInfo::new("s1".into(), "Test".into());
        store.create(&info).await.unwrap();

        store.delete("s1").await.unwrap();

        let retrieved = store.get("s1").await.unwrap();
        assert!(retrieved.is_none());
    }
}
