use agent_core::tools::{ToolExecutionErrorKind, ToolExecutionPolicy, ToolParallelMode};

#[test]
fn default_tool_policy_is_parallel_safe_without_retry() {
    let p = ToolExecutionPolicy::default();
    assert!(matches!(p.parallel_mode, ToolParallelMode::ParallelSafe));
    assert!(p.timeout_ms.is_none());
    assert!(p.retry.is_none());
}

#[test]
fn tool_error_kind_roundtrip_debug() {
    let kind = ToolExecutionErrorKind::Transient;
    assert_eq!(format!("{kind:?}"), "Transient");
}
