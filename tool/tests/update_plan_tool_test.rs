use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn update_plan_accepts_valid_plan_and_emits_structured_output() {
    use tool::{Tool, ToolContext};
    let tool = tool::builtin::update_plan::UpdatePlanTool;
    let result = tool
        .execute(
            ToolContext::new("s1", "t1", CancellationToken::new()),
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
    assert_eq!(
        result.output["plan"]["tasks"][0]["title"],
        "Write failing test"
    );
    assert_eq!(result.output["plan"]["tasks"][0]["status"], "in_progress");
    assert_eq!(result.output["plan"]["is_streaming"], true);
}

#[tokio::test]
async fn update_plan_rejects_multiple_in_progress_steps() {
    use tool::{Tool, ToolContext};
    let tool = tool::builtin::update_plan::UpdatePlanTool;
    let err = tool
        .execute(
            ToolContext::new("s1", "t1", CancellationToken::new()),
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

#[tokio::test]
async fn update_plan_accepts_zero_in_progress_steps() {
    // Relaxed rule: 0 in_progress is allowed (even with pending steps)
    use tool::{Tool, ToolContext};
    let tool = tool::builtin::update_plan::UpdatePlanTool;
    let result = tool
        .execute(
            ToolContext::new("s1", "t1", CancellationToken::new()),
            serde_json::json!({
                "plan": [
                    { "step": "Step A", "status": "pending" },
                    { "step": "Step B", "status": "pending" }
                ]
            }),
        )
        .await
        .expect("0 in_progress should be allowed");

    assert!(!result.is_error);
    assert_eq!(result.output["plan"]["tasks"][0]["status"], "pending");
}

#[tokio::test]
async fn update_plan_rejects_empty_step() {
    use tool::{Tool, ToolContext};
    let tool = tool::builtin::update_plan::UpdatePlanTool;
    let err = tool
        .execute(
            ToolContext::new("s1", "t1", CancellationToken::new()),
            serde_json::json!({
                "plan": [
                    { "step": "   ", "status": "pending" }
                ]
            }),
        )
        .await
        .expect_err("empty step should be rejected");

    assert!(err.to_string().contains("empty"));
}
