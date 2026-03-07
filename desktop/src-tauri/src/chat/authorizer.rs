use argus_core::{Builtin, ToolCall};
use async_trait::async_trait;
use turn::{AuthorizationDecision, ToolAuthorizer, TurnError};

#[derive(Debug, Default, Clone, Copy)]
pub struct AllowListedToolAuthorizer;

#[async_trait]
impl ToolAuthorizer for AllowListedToolAuthorizer {
    async fn authorize(&self, call: &ToolCall) -> Result<AuthorizationDecision, TurnError> {
        Ok(match call {
            ToolCall::Builtin(call)
                if matches!(call.builtin, Builtin::Read | Builtin::Glob | Builtin::Grep) =>
            {
                AuthorizationDecision::Allow
            }
            _ => AuthorizationDecision::Deny,
        })
    }
}
