use async_trait::async_trait;
use argus_core::ToolCall;

use crate::TurnError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    pub request_id: String,
    pub tool_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationDecision {
    Allow,
    Deny,
    Ask(PermissionRequest),
}

#[async_trait]
pub trait ToolAuthorizer: Send + Sync {
    async fn authorize(&self, call: &ToolCall) -> Result<AuthorizationDecision, TurnError>;
}
