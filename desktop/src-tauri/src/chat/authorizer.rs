use agent::AgentToolSurface;
use argus_core::ToolCall;
use async_trait::async_trait;
use turn::{AuthorizationDecision, PermissionRequest, ToolAuthorizer, TurnError};

#[derive(Debug, Clone)]
pub struct AllowListedToolAuthorizer {
    surface: AgentToolSurface,
}

impl AllowListedToolAuthorizer {
    pub fn new(surface: AgentToolSurface) -> Self {
        Self { surface }
    }
}

#[async_trait]
impl ToolAuthorizer for AllowListedToolAuthorizer {
    async fn authorize(&self, call: &ToolCall) -> Result<AuthorizationDecision, TurnError> {
        Ok(match call {
            ToolCall::Builtin(call) if self.surface.allows_builtin(&call.builtin) => {
                AuthorizationDecision::Allow
            }
            _ => AuthorizationDecision::Ask(PermissionRequest {
                request_id: format!("perm-{}", tool_call_id(call)),
                tool_call_id: tool_call_id(call).to_string(),
            }),
        })
    }
}

fn tool_call_id(call: &ToolCall) -> &str {
    match call {
        ToolCall::FunctionCall { call_id, .. } => call_id,
        ToolCall::Builtin(call) => &call.call_id,
        ToolCall::Mcp(call) => &call.id,
    }
}
