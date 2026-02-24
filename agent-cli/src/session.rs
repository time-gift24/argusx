use anyhow::{anyhow, Result};
use async_trait::async_trait;

#[async_trait]
pub trait SessionGateway: Send + Sync {
    async fn create_session(&self) -> Result<String>;
    async fn session_exists(&self, session_id: &str) -> Result<bool>;
}

pub async fn resolve_session_id<G: SessionGateway>(
    gateway: &G,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(session_id) = requested {
        let trimmed = session_id.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("invalid session id: empty string"));
        }
        if gateway.session_exists(trimmed).await? {
            return Ok(trimmed.to_string());
        }
        return Err(anyhow!("session not found: {trimmed}"));
    }

    gateway.create_session().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeGateway {
        existing: Vec<String>,
    }

    impl FakeGateway {
        fn with_existing(mut self, ids: &[&str]) -> Self {
            self.existing = ids.iter().map(|s| s.to_string()).collect();
            self
        }
    }

    #[async_trait]
    impl SessionGateway for FakeGateway {
        async fn create_session(&self) -> Result<String> {
            Ok("new-session".to_string())
        }

        async fn session_exists(&self, session_id: &str) -> Result<bool> {
            Ok(self.existing.iter().any(|s| s == session_id))
        }
    }

    #[tokio::test]
    async fn no_session_arg_creates_new_session() {
        let gateway = FakeGateway::default();
        let id = resolve_session_id(&gateway, None).await.unwrap();
        assert_eq!(id, "new-session");
    }

    #[tokio::test]
    async fn provided_session_must_exist() {
        let gateway = FakeGateway::default().with_existing(&["s-1"]);
        let id = resolve_session_id(&gateway, Some("s-1")).await.unwrap();
        assert_eq!(id, "s-1");
    }

    #[tokio::test]
    async fn missing_provided_session_returns_error() {
        let gateway = FakeGateway::default();
        let err = resolve_session_id(&gateway, Some("missing"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("session not found"));
    }

    #[tokio::test]
    async fn empty_session_string_returns_error() {
        let gateway = FakeGateway::default();
        let err = resolve_session_id(&gateway, Some("")).await.unwrap_err();
        assert!(err.to_string().contains("invalid session id"));
    }

    #[tokio::test]
    async fn whitespace_only_session_returns_error() {
        let gateway = FakeGateway::default();
        let err = resolve_session_id(&gateway, Some("   ")).await.unwrap_err();
        assert!(err.to_string().contains("invalid session id"));
    }
}
