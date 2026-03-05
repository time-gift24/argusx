use agent_core::tools::{
    ToolCatalog, ToolExecutionContext, ToolExecutionError, ToolExecutionErrorKind, ToolExecutor,
};
use agent_core::{ToolCall, ToolResult as CoreToolResult};
use async_trait::async_trait;

use crate::{GlobTool, GrepTool, ReadTool, ToolContext, ToolError, ToolRegistry, UpdatePlanTool};

pub struct AgentToolRuntime {
    registry: ToolRegistry,
}

impl AgentToolRuntime {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub async fn default_with_builtins() -> Self {
        let registry = ToolRegistry::new();
        // Register read-only filesystem tools with default allowed root (current directory)
        let read_tool = ReadTool::default().expect("Failed to create default ReadTool");
        let glob_tool = GlobTool::default().expect("Failed to create default GlobTool");
        let grep_tool = GrepTool::default().expect("Failed to create default GrepTool");
        let update_plan_tool = UpdatePlanTool;
        registry.register(read_tool).await;
        registry.register(glob_tool).await;
        registry.register(grep_tool).await;
        registry.register(update_plan_tool).await;
        Self { registry }
    }

    /// Register an external tool
    pub async fn register_tool<T: crate::Tool + 'static>(&self, tool: T) {
        self.registry.register(tool).await;
    }

    /// Get the underlying registry for advanced tool registration
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

#[async_trait]
impl ToolCatalog for AgentToolRuntime {
    async fn list_tools(&self) -> Vec<agent_core::tools::ToolSpec> {
        self.registry
            .list()
            .await
            .into_iter()
            .map(map_spec)
            .collect()
    }

    async fn tool_spec(&self, name: &str) -> Option<agent_core::tools::ToolSpec> {
        self.registry
            .get(name)
            .await
            .map(|tool| map_spec(tool.spec()))
    }
}

#[async_trait]
impl ToolExecutor for AgentToolRuntime {
    async fn execute_tool(
        &self,
        call: ToolCall,
        ctx: ToolExecutionContext,
    ) -> Result<CoreToolResult, ToolExecutionError> {
        let out = self
            .registry
            .call(
                &call.tool_name,
                call.arguments,
                ToolContext {
                    session_id: ctx.session_id,
                    turn_id: ctx.turn_id,
                },
            )
            .await
            .map_err(map_error)?;

        Ok(CoreToolResult {
            call_id: call.call_id,
            output: out.output,
            is_error: out.is_error,
        })
    }
}

fn map_spec(spec: crate::ToolSpec) -> agent_core::tools::ToolSpec {
    agent_core::tools::ToolSpec {
        name: spec.name,
        description: spec.description,
        input_schema: spec.input_schema,
        execution_policy: agent_core::tools::ToolExecutionPolicy::default(),
    }
}

fn map_error(err: ToolError) -> ToolExecutionError {
    // Classify errors based on their nature:
    // - User errors: invalid arguments, not found, policy denials (access denied)
    // - Runtime errors: actual IO failures, system errors
    let (kind, message) = match err {
        ToolError::NotFound(msg) => (ToolExecutionErrorKind::User, msg),
        ToolError::InvalidArgs(msg) => (ToolExecutionErrorKind::User, msg),
        ToolError::ExecutionFailed(msg) => {
            // Check if it's a policy denial (access denied, not found, etc.)
            let is_policy = msg.starts_with("Access denied:")
                || msg.starts_with("Not found:")
                || msg.starts_with("Invalid root:")
                || msg.starts_with("Invalid path:");
            if is_policy {
                (ToolExecutionErrorKind::User, msg)
            } else {
                (ToolExecutionErrorKind::Runtime, msg)
            }
        }
        ToolError::Io(msg) => (ToolExecutionErrorKind::Runtime, msg.to_string()),
    };

    ToolExecutionError {
        kind,
        message,
        retry_after_ms: None,
    }
}
