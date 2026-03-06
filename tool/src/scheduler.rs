use std::collections::BTreeMap;
use std::sync::Arc;

use argus_core::{Builtin, BuiltinToolCall};
use tokio::sync::Semaphore;

use crate::{Tool, ToolContext, ToolError, ToolResult};

pub struct BuiltinRegistration {
    pub builtin: Builtin,
    pub tool: Arc<dyn Tool>,
    pub policy: EffectiveToolPolicy,
}

impl BuiltinRegistration {
    pub fn new(builtin: Builtin, tool: Arc<dyn Tool>, policy: EffectiveToolPolicy) -> Self {
        Self {
            builtin,
            tool,
            policy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveToolPolicy {
    pub allow_parallel: bool,
    pub max_concurrency: usize,
}

pub struct ToolScheduler {
    builtin_tools: BTreeMap<String, Arc<dyn Tool>>,
    builtin_gates: BTreeMap<String, Arc<Semaphore>>,
}

impl ToolScheduler {
    pub fn new(
        registrations: impl IntoIterator<Item = BuiltinRegistration>,
    ) -> Result<Self, ToolError> {
        let mut builtin_tools = BTreeMap::new();
        let mut builtin_gates = BTreeMap::new();

        for registration in registrations {
            let name = registration.builtin.canonical_name().to_string();
            if builtin_tools.contains_key(&name) {
                return Err(ToolError::ExecutionFailed(format!(
                    "duplicate builtin registration: {name}"
                )));
            }

            builtin_gates.insert(
                name.clone(),
                Arc::new(Semaphore::new(effective_limit(registration.policy))),
            );
            builtin_tools.insert(name, registration.tool);
        }

        Ok(Self {
            builtin_tools,
            builtin_gates,
        })
    }

    pub async fn execute_builtin(
        &self,
        call: BuiltinToolCall,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let builtin_name = call.builtin.canonical_name().to_string();
        let tool = self
            .builtin_tools
            .get(&builtin_name)
            .cloned()
            .ok_or_else(|| ToolError::NotFound(builtin_name.clone()))?;
        let gate = self
            .builtin_gates
            .get(&builtin_name)
            .cloned()
            .ok_or_else(|| ToolError::NotFound(builtin_name.clone()))?;

        let _permit = gate.acquire_owned().await.map_err(|_| {
            ToolError::ExecutionFailed(format!("scheduler gate closed for {builtin_name}"))
        })?;

        let args = serde_json::from_str(&call.arguments_json).map_err(|err| {
            ToolError::InvalidArgs(format!("invalid builtin arguments json: {err}"))
        })?;

        tool.execute(ctx, args).await
    }
}

fn effective_limit(policy: EffectiveToolPolicy) -> usize {
    if !policy.allow_parallel {
        return 1;
    }

    policy.max_concurrency.max(1)
}
