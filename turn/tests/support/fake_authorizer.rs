use async_trait::async_trait;
use argus_core::ToolCall;
use turn::{AuthorizationDecision, ToolAuthorizer, TurnError};

pub struct FakeAuthorizer;

#[async_trait]
impl ToolAuthorizer for FakeAuthorizer {
    async fn authorize(&self, _call: &ToolCall) -> Result<AuthorizationDecision, TurnError> {
        Ok(AuthorizationDecision::Allow)
    }
}
