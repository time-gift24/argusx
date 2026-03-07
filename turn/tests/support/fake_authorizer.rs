use std::{collections::BTreeMap, sync::Arc};

use argus_core::ToolCall;
use async_trait::async_trait;
use tokio::sync::Mutex;
use turn::{AuthorizationDecision, ToolAuthorizer, TurnError};

pub struct FakeAuthorizer {
    decisions: Arc<Mutex<BTreeMap<String, AuthorizationDecision>>>,
}

impl FakeAuthorizer {
    pub fn new(decisions: impl IntoIterator<Item = (String, AuthorizationDecision)>) -> Self {
        Self {
            decisions: Arc::new(Mutex::new(decisions.into_iter().collect())),
        }
    }
}

#[async_trait]
impl ToolAuthorizer for FakeAuthorizer {
    async fn authorize(&self, call: &ToolCall) -> Result<AuthorizationDecision, TurnError> {
        let call_id = match call {
            ToolCall::FunctionCall { call_id, .. } => call_id.clone(),
            ToolCall::Builtin(call) => call.call_id.clone(),
            ToolCall::Mcp(call) => call.id.clone(),
        };

        Ok(self
            .decisions
            .lock()
            .await
            .get(&call_id)
            .cloned()
            .unwrap_or(AuthorizationDecision::Allow))
    }
}

impl Default for FakeAuthorizer {
    fn default() -> Self {
        Self::new(std::iter::empty())
    }
}
