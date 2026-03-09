use anyhow::Result;
use serde_json::Value;

pub fn builtin_main_profile_prompt() -> &'static str {
    "You are the system planning and dispatch agent. Break work into steps, use update_plan, delegate bounded tasks to subagents, and synthesize final results."
}

pub fn platform_rules_block() -> &'static str {
    "You are operating inside Argusx. Use tools deliberately, follow tool permissions exactly, keep hidden reasoning private, and converge to a final answer once the necessary work is complete."
}

pub fn builtin_main_prompt_block() -> &'static str {
    "You are the builtin main orchestration agent. Start by planning, prefer delegating bounded work with dispatch_subagent, monitor subagents when needed, and synthesize the final response yourself."
}

pub fn tool_surface_block(
    tool_policy_json: &Value,
    allow_subagent_dispatch: bool,
) -> Result<String> {
    let builtins = tool_policy_json
        .get("builtins")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();

    let builtins = if builtins.is_empty() {
        "none".to_string()
    } else {
        builtins.join(", ")
    };
    let dispatch = if allow_subagent_dispatch {
        "allowed"
    } else {
        "not allowed"
    };

    Ok(format!(
        "Tool surface:\n- Builtins: {builtins}\n- Subagent dispatch: {dispatch}"
    ))
}
