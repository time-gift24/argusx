use async_trait::async_trait;
use agent_cli::session::{resolve_session_id, SessionGateway};

#[derive(Default)]
struct FakeGateway;

#[async_trait]
impl SessionGateway for FakeGateway {
    async fn create_session(&self) -> anyhow::Result<String> {
        Ok("new".into())
    }
    async fn session_exists(&self, _session_id: &str) -> anyhow::Result<bool> {
        Ok(false)
    }
}

#[tokio::test]
async fn resume_mode_rejects_missing_session() {
    let err = resolve_session_id(&FakeGateway, Some("missing")).await.unwrap_err();
    assert!(err.to_string().contains("session not found: missing"));
}
