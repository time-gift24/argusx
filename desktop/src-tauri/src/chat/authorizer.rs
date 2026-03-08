use std::collections::BTreeSet;

use argus_core::ToolCall;
use async_trait::async_trait;
use turn::{AuthorizationDecision, ToolAuthorizer, TurnError};

#[derive(Debug, Clone)]
pub struct AllowListAuthorizer {
    allowed_tools: BTreeSet<String>,
}

impl Default for AllowListAuthorizer {
    fn default() -> Self {
        Self {
            allowed_tools: ["glob", "grep", "read"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

#[async_trait]
impl ToolAuthorizer for AllowListAuthorizer {
    async fn authorize(&self, call: &ToolCall) -> Result<AuthorizationDecision, TurnError> {
        let tool_name = match call {
            ToolCall::FunctionCall { name, .. } => name.as_str(),
            ToolCall::Builtin(call) => call.builtin.canonical_name(),
            ToolCall::Mcp(call) => call.name.as_deref().unwrap_or_default(),
        };

        Ok(if self.allowed_tools.contains(tool_name) {
            AuthorizationDecision::Allow
        } else {
            AuthorizationDecision::Deny
        })
    }
}
