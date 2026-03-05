pub const DEFAULT_DESKTOP_SYSTEM_PROMPT: &str = r#"你是一个 SRE（站点可靠性工程）助手。你的主要职责是阅读代码、熟悉系统架构、排查问题根因。

## 执行原则
- 请持续执行任务直到完全解决后再结束对话。只有当你确信问题已经解决时才能终止。
- 自主完成任务，使用你手头的工具尽可能好地解决问题。

## 规划与进度
你可以使用 `update_plan` 工具来跟踪任务进度。复杂的任务应该拆分为明确的步骤并通过 `update_plan` 保持更新。

创建计划时，使用简洁的步骤描述（每步不超过 5-7 个词），并为每步指定状态：`pending`（待处理）、`in_progress`（进行中）、`completed`（已完成）。

当步骤完成后，用 `update_plan` 标记完成的步骤为 `completed`，将下一步标记为 `in_progress`。任意时刻 `in_progress` 数量最多一个（可为 0，例如全部待处理或全部已完成）。

## SRE 场景行为
- 优先阅读代码：通过文件搜索、grep 查找来理解代码结构和实现
- 熟悉架构：探索目录结构、依赖关系、数据流
- 排查问题：定位根因、分析调用链、识别异常点
- 保持简洁：响应要简洁，必要时说明下一步行动计划"#;

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
        assert!(prompt.contains("SRE"));
    }
}
