use std::{collections::BTreeMap, sync::Arc, time::Duration};

use argus_core::ToolCall;
use async_trait::async_trait;
use serde_json::json;
use tokio::{sync::Mutex, time::sleep};
use tool::{ToolContext, ToolResult};
use turn::{ToolRunner, TurnError};

pub struct FakeToolRunner {
    plans: Arc<Mutex<BTreeMap<String, PlannedToolResult>>>,
}

struct PlannedToolResult {
    delay_ms: u64,
    result: ToolResult,
}

impl FakeToolRunner {
    pub fn new(plans: impl IntoIterator<Item = (String, u64, ToolResult)>) -> Self {
        let plans = plans
            .into_iter()
            .map(|(call_id, delay_ms, result)| (call_id, PlannedToolResult { delay_ms, result }))
            .collect();
        Self {
            plans: Arc::new(Mutex::new(plans)),
        }
    }
}

#[async_trait]
impl ToolRunner for FakeToolRunner {
    async fn execute(&self, call: ToolCall, _ctx: ToolContext) -> Result<ToolResult, TurnError> {
        let call_id = match call {
            ToolCall::FunctionCall { call_id, .. } => call_id,
            ToolCall::Builtin(call) => call.call_id,
            ToolCall::Mcp(call) => call.id,
        };

        let plan = {
            let mut plans = self.plans.lock().await;
            plans.remove(&call_id)
        };

        if let Some(plan) = plan {
            sleep(Duration::from_millis(plan.delay_ms)).await;
            return Ok(plan.result);
        }

        Ok(ToolResult::ok(json!({"ok": true})))
    }
}

impl Default for FakeToolRunner {
    fn default() -> Self {
        Self::new(std::iter::empty())
    }
}
