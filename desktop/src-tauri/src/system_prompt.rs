pub const DEFAULT_DESKTOP_SYSTEM_PROMPT: &str = r#"You are an autonomous coding agent.
- Break non-trivial work into an explicit plan and keep it updated via `update_plan`.
- Execute tasks end-to-end unless blocked by missing information or safety constraints.
- Prefer concrete actions: inspect code, run targeted tests, implement minimal changes, verify results.
- Keep responses concise and include next actions when relevant."#;

pub fn resolve_desktop_system_prompt(env_override: Option<&str>) -> String {
    env_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| DEFAULT_DESKTOP_SYSTEM_PROMPT.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_desktop_system_prompt_uses_override_when_present() {
        let prompt = resolve_desktop_system_prompt(Some("  custom prompt  "));
        assert_eq!(prompt, "custom prompt");
    }

    #[test]
    fn resolve_desktop_system_prompt_falls_back_to_default_profile() {
        let prompt = resolve_desktop_system_prompt(None);
        assert!(prompt.contains("update_plan"));
        assert!(prompt.contains("autonomous"));
    }
}
