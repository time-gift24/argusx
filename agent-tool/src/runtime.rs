use agent_core::tools::{
    ToolCatalog, ToolExecutionContext, ToolExecutionError, ToolExecutionErrorKind, ToolExecutor,
};
use agent_core::{ToolCall, ToolResult as CoreToolResult};
use async_trait::async_trait;

use crate::{
    DomainCookiesTool, ReadFileTool, ShellTool, ToolContext, ToolError, ToolRegistry,
    UpdatePlanTool,
};

pub struct AgentToolRuntime {
    registry: ToolRegistry,
}

impl AgentToolRuntime {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub async fn default_with_builtins() -> Self {
        let registry = ToolRegistry::new();
        registry.register(ReadFileTool).await;
        registry.register(ShellTool).await;
        registry.register(DomainCookiesTool::from_env()).await;
        registry.register(UpdatePlanTool).await;
        Self { registry }
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
    let kind = match err {
        ToolError::NotFound(_) | ToolError::InvalidArgs(_) => ToolExecutionErrorKind::User,
        ToolError::ExecutionFailed(_) | ToolError::Io(_) => ToolExecutionErrorKind::Runtime,
    };

    ToolExecutionError {
        kind,
        message: err.to_string(),
        retry_after_ms: None,
    }
}
