use anyhow::Result;

use crate::{
    ResolvedAgentExecution, ThreadAgentSnapshot,
    prompts::{builtin_main_prompt_block, platform_rules_block, tool_surface_block},
};

#[derive(Debug, Default, Clone, Copy)]
pub struct AgentExecutionResolver;

impl AgentExecutionResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(
        &self,
        session_prompt: &str,
        snapshot: &ThreadAgentSnapshot,
    ) -> Result<ResolvedAgentExecution> {
        let mut blocks = vec![platform_rules_block().to_string()];

        let session_prompt = session_prompt.trim();
        if !session_prompt.is_empty() {
            blocks.push(session_prompt.to_string());
        }

        if snapshot.profile_id == "builtin-main" {
            blocks.push(builtin_main_prompt_block().to_string());
        }

        let profile_prompt = snapshot.system_prompt_snapshot.trim();
        if !profile_prompt.is_empty() {
            blocks.push(profile_prompt.to_string());
        }

        blocks.push(tool_surface_block(
            &snapshot.tool_policy_snapshot_json,
            snapshot.allow_subagent_dispatch_snapshot,
        )?);

        Ok(ResolvedAgentExecution {
            system_prompt: blocks.join("\n\n"),
            tool_policy: snapshot.tool_policy_snapshot_json.clone(),
            model_override: (!snapshot.model_config_snapshot_json.is_null())
                .then(|| snapshot.model_config_snapshot_json.clone()),
            allow_subagent_dispatch: snapshot.allow_subagent_dispatch_snapshot,
        })
    }
}
