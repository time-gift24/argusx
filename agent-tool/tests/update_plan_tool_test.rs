#[tokio::test]
async fn update_plan_accepts_valid_plan_and_emits_structured_output() {
    use agent_tool::{Tool, ToolContext};
    let tool = agent_tool::builtin::update_plan::UpdatePlanTool;
    let result = tool
        .execute(
            ToolContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
            },
            serde_json::json!({
                "explanation": "Starting execution",
                "plan": [
                    { "step": "Write failing test", "status": "in_progress" },
                    { "step": "Implement minimal fix", "status": "pending" }
                ]
            }),
        )
        .await
        .expect("valid plan should pass");

    assert!(!result.is_error);
    assert_eq!(result.output["plan"]["tasks"][0]["title"], "Write failing test");
    assert_eq!(result.output["plan"]["tasks"][0]["status"], "in_progress");
    assert_eq!(result.output["plan"]["is_streaming"], true);
}

#[tokio::test]
async fn update_plan_rejects_multiple_in_progress_steps() {
    use agent_tool::{Tool, ToolContext};
    let tool = agent_tool::builtin::update_plan::UpdatePlanTool;
    let err = tool
        .execute(
            ToolContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
            },
            serde_json::json!({
                "plan": [
                    { "step": "Step A", "status": "in_progress" },
                    { "step": "Step B", "status": "in_progress" }
                ]
            }),
        )
        .await
        .expect_err("must reject invalid in_progress multiplicity");

    assert!(err.to_string().contains("in_progress"));
}
