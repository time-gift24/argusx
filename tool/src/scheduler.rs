use std::collections::BTreeMap;
use std::sync::Arc;

use argus_core::{BuiltinToolCall, McpCall, McpCallType, ToolCall};
use serde_json::json;
use tokio::sync::Semaphore;

pub use crate::catalog::{BuiltinRegistration, EffectiveToolPolicy, McpRegistration};
use crate::{Tool, ToolContext, ToolError, ToolResult, mcp::McpError};

pub struct ToolScheduler {
    builtin_tools: BTreeMap<String, Arc<dyn Tool>>,
    builtin_gates: BTreeMap<String, Arc<Semaphore>>,
    mcp_clients: BTreeMap<String, Arc<crate::mcp::McpClient>>,
    mcp_gates: BTreeMap<String, Arc<Semaphore>>,
}

impl ToolScheduler {
    pub fn new(
        registrations: impl IntoIterator<Item = BuiltinRegistration>,
    ) -> Result<Self, ToolError> {
        Self::from_parts(registrations, std::iter::empty())
    }

    pub fn from_parts(
        builtin_registrations: impl IntoIterator<Item = BuiltinRegistration>,
        mcp_registrations: impl IntoIterator<Item = McpRegistration>,
    ) -> Result<Self, ToolError> {
        let mut builtin_tools = BTreeMap::new();
        let mut builtin_gates = BTreeMap::new();
        let mut mcp_clients = BTreeMap::new();
        let mut mcp_gates = BTreeMap::new();

        for registration in builtin_registrations {
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

        for registration in mcp_registrations {
            if mcp_clients.contains_key(&registration.server_label) {
                return Err(ToolError::ExecutionFailed(format!(
                    "duplicate mcp registration: {}",
                    registration.server_label
                )));
            }

            mcp_gates.insert(
                registration.server_label.clone(),
                Arc::new(Semaphore::new(effective_limit(registration.policy))),
            );
            mcp_clients.insert(registration.server_label, registration.client);
        }

        Ok(Self {
            builtin_tools,
            builtin_gates,
            mcp_clients,
            mcp_gates,
        })
    }

    pub async fn execute(&self, call: ToolCall, ctx: ToolContext) -> Result<ToolResult, ToolError> {
        match call {
            ToolCall::Builtin(call) => self.execute_builtin(call, ctx).await,
            ToolCall::Mcp(call) => self.execute_mcp(call).await,
            ToolCall::FunctionCall { name, .. } => Err(ToolError::Unsupported(format!(
                "function call execution is not implemented for `{name}`"
            ))),
        }
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

    pub async fn execute_mcp(&self, call: McpCall) -> Result<ToolResult, ToolError> {
        let server_label = call
            .server_label
            .clone()
            .ok_or_else(|| ToolError::InvalidArgs("missing MCP server label".to_string()))?;
        let client = self
            .mcp_clients
            .get(&server_label)
            .cloned()
            .ok_or_else(|| ToolError::NotFound(server_label.clone()))?;
        let gate = self
            .mcp_gates
            .get(&server_label)
            .cloned()
            .ok_or_else(|| ToolError::NotFound(server_label.clone()))?;

        let _permit = gate.acquire_owned().await.map_err(|_| {
            ToolError::ExecutionFailed(format!("scheduler gate closed for {server_label}"))
        })?;

        match call.mcp_type {
            McpCallType::McpListTools => {
                let tools = client.list_tools().await.map_err(map_mcp_error)?;
                Ok(ToolResult::ok(json!({ "tools": tools })))
            }
            McpCallType::McpCall => {
                let name = call
                    .name
                    .as_deref()
                    .ok_or_else(|| ToolError::InvalidArgs("missing MCP tool name".to_string()))?;
                let arguments_json = call.arguments_json.as_deref().unwrap_or("{}");
                let output = client
                    .call_tool(name, arguments_json)
                    .await
                    .map_err(map_mcp_error)?;
                let is_error = output
                    .get("isError")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);

                Ok(ToolResult { output, is_error })
            }
            McpCallType::Unknown(kind) => Err(ToolError::Unsupported(format!(
                "unsupported MCP call type `{kind}`"
            ))),
        }
    }
}

fn effective_limit(policy: EffectiveToolPolicy) -> usize {
    if !policy.allow_parallel {
        return 1;
    }

    policy.max_concurrency.max(1)
}

fn map_mcp_error(err: McpError) -> ToolError {
    match err {
        McpError::Json(err) => ToolError::InvalidArgs(err.to_string()),
        other => ToolError::ExecutionFailed(other.to_string()),
    }
}
