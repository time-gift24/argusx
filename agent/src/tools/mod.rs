use std::sync::Arc;

use anyhow::{bail, Result};
use argus_core::Builtin;
use tool::{
    scheduler::{BuiltinRegistration, EffectiveToolPolicy},
    GlobTool, GrepTool, ReadTool, Tool, ToolSpec, UpdatePlanTool,
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

        if let Some(entries) = tool_policy_json
            .get("builtins")
            .and_then(serde_json::Value::as_array)
        {
            for entry in entries {
                let name = entry
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("builtin names must be strings"))?;
                let builtin = Builtin::from_name(name)
                    .ok_or_else(|| anyhow::anyhow!("unsupported builtin `{name}`"))?;
                if is_executable_builtin(&builtin) && !builtins.contains(&builtin) {
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
        self.builtins
            .iter()
            .any(|builtin| builtin.canonical_name() == name)
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
        other => bail!(
            "unsupported builtin in agent tool surface: {}",
            other.canonical_name()
        ),
    })
}

fn is_executable_builtin(builtin: &Builtin) -> bool {
    matches!(
        builtin,
        Builtin::Read | Builtin::Glob | Builtin::Grep | Builtin::UpdatePlan
    )
}
