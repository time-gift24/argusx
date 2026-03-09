use anyhow::Result;
use serde_json::Value;

use crate::AgentToolSurface;

pub fn builtin_main_profile_prompt() -> &'static str {
    "You are the system planning agent. Break work into steps, use update_plan, inspect the workspace with low-risk tools when needed, and synthesize final results."
}

pub fn platform_rules_block() -> &'static str {
    "You are operating inside Argusx. Use tools deliberately, follow tool permissions exactly, keep hidden reasoning private, and converge to a final answer once the necessary work is complete."
}

pub fn builtin_main_prompt_block() -> &'static str {
    "You are the builtin main orchestration agent. Start by planning, use the available low-risk tools deliberately, and synthesize the final response yourself."
}

pub fn tool_surface_block(
    tool_policy_json: &Value,
    allow_subagent_dispatch: bool,
) -> Result<String> {
    let surface = AgentToolSurface::from_policy(tool_policy_json)?;
    let builtins = surface
        .builtins()
        .iter()
        .map(|builtin| builtin.canonical_name())
        .collect::<Vec<_>>();

    let builtins = if builtins.is_empty() {
        "none".to_string()
    } else {
        builtins.join(", ")
    };
    let dispatch = if allow_subagent_dispatch && surface.has_builtin("dispatch_subagent") {
        "allowed"
    } else {
        "not allowed"
    };

    Ok(format!(
        "Tool surface:\n- Builtins: {builtins}\n- Subagent dispatch: {dispatch}"
    ))
}
