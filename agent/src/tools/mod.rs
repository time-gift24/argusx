use std::sync::Arc;

use anyhow::{Result, bail};
use argus_core::Builtin;
use async_trait::async_trait;
use serde_json::json;
use tool::{
    GlobTool, GrepTool, ReadTool, Tool, ToolContext, ToolError, ToolResult, ToolSpec,
    UpdatePlanTool,
    scheduler::{BuiltinRegistration, EffectiveToolPolicy},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentToolSurface {
    builtins: Vec<Builtin>,
}

pub fn build_agent_tool_surface(tool_policy_json: serde_json::Value) -> Result<AgentToolSurface> {
    AgentToolSurface::from_policy(&tool_policy_json)
}

impl AgentToolSurface {
    pub fn from_policy(tool_policy_json: &serde_json::Value) -> Result<Self> {
        let mut builtins = Vec::new();

        if let Some(entries) = tool_policy_json.get("builtins").and_then(serde_json::Value::as_array)
        {
            for entry in entries {
                let name = entry
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("builtin names must be strings"))?;
                let builtin = Builtin::from_name(name)
                    .ok_or_else(|| anyhow::anyhow!("unsupported builtin `{name}`"))?;
                if !builtins.contains(&builtin) {
                    builtins.push(builtin);
                }
            }
        }

        Ok(Self { builtins })
    }

    pub fn builtins(&self) -> &[Builtin] {
        &self.builtins
    }

    pub fn has_builtin(&self, name: &str) -> bool {
        self.builtins.iter().any(|builtin| builtin.canonical_name() == name)
    }

    pub fn allows_builtin(&self, builtin: &Builtin) -> bool {
        self.builtins.iter().any(|allowed| allowed == builtin)
    }

    pub fn builtin_registrations_from_current_dir(&self) -> Result<Vec<BuiltinRegistration>> {
        let policy = EffectiveToolPolicy {
            allow_parallel: true,
            max_concurrency: 4,
        };

        self.builtins
            .iter()
            .map(|builtin| {
                Ok(BuiltinRegistration::new(
                    builtin.clone(),
                    materialize_builtin_tool(builtin)?,
                    policy,
                ))
            })
            .collect()
    }

    pub fn tool_specs_from_current_dir(&self) -> Result<Vec<ToolSpec>> {
        self.builtins
            .iter()
            .map(|builtin| Ok(materialize_builtin_tool(builtin)?.spec()))
            .collect()
    }
}

fn materialize_builtin_tool(builtin: &Builtin) -> Result<Arc<dyn Tool>> {
    Ok(match builtin {
        Builtin::Read => Arc::new(ReadTool::from_current_dir()?) as Arc<dyn Tool>,
        Builtin::Glob => Arc::new(GlobTool::from_current_dir()?) as Arc<dyn Tool>,
        Builtin::Grep => Arc::new(GrepTool::from_current_dir()?) as Arc<dyn Tool>,
        Builtin::UpdatePlan => Arc::new(UpdatePlanTool) as Arc<dyn Tool>,
        Builtin::DispatchSubagent => Arc::new(PendingBuiltinTool::dispatch_subagent()) as Arc<dyn Tool>,
        Builtin::ListSubagentDispatches => {
            Arc::new(PendingBuiltinTool::list_subagent_dispatches()) as Arc<dyn Tool>
        }
        Builtin::GetSubagentDispatch => {
            Arc::new(PendingBuiltinTool::get_subagent_dispatch()) as Arc<dyn Tool>
        }
        other => bail!("unsupported builtin in agent tool surface: {}", other.canonical_name()),
    })
}

struct PendingBuiltinTool {
    spec: ToolSpec,
}

impl PendingBuiltinTool {
    fn dispatch_subagent() -> Self {
        Self {
            spec: ToolSpec {
                name: "dispatch_subagent".into(),
                description: "Dispatch a task to another agent thread and wait for the child turn to reach a terminal state.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "agent_profile_id": { "type": "string" },
                        "task": { "type": "string" },
                        "context": { "type": "object" },
                        "title": { "type": "string" }
                    },
                    "required": ["agent_profile_id", "task"]
                }),
            },
        }
    }

    fn list_subagent_dispatches() -> Self {
        Self {
            spec: ToolSpec {
                name: "list_subagent_dispatches".into(),
                description: "List tracked subagent dispatches for the current session or parent thread.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "parent_thread_id": { "type": "string" },
                        "status": { "type": "string" }
                    }
                }),
            },
        }
    }

    fn get_subagent_dispatch() -> Self {
        Self {
            spec: ToolSpec {
                name: "get_subagent_dispatch".into(),
                description: "Read the latest status and summary for a tracked subagent dispatch.".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "dispatch_id": { "type": "string" }
                    },
                    "required": ["dispatch_id"]
                }),
            },
        }
    }
}

#[async_trait]
impl Tool for PendingBuiltinTool {
    fn name(&self) -> &str {
        &self.spec.name
    }

    fn description(&self) -> &str {
        &self.spec.description
    }

    fn spec(&self) -> ToolSpec {
        self.spec.clone()
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::ExecutionFailed(format!(
            "{} is not implemented yet",
            self.spec.name
        )))
    }
}
