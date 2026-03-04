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

#[tokio::test]
async fn update_plan_accepts_zero_in_progress_steps() {
    // Relaxed rule: 0 in_progress is allowed (even with pending steps)
    use agent_tool::{Tool, ToolContext};
    let tool = agent_tool::builtin::update_plan::UpdatePlanTool;
    let result = tool
        .execute(
            ToolContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
            },
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
                    { "step": "   ", "status": "pending" }
                ]
            }),
        )
        .await
        .expect_err("empty step should be rejected");

    assert!(err.to_string().contains("empty"));
}

#[tokio::test]
async fn update_plan_accepts_five_task_statuses() {
    use agent_tool::{Tool, ToolContext};
    let tool = agent_tool::builtin::update_plan::UpdatePlanTool;
    let result = tool
        .execute(
            ToolContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
            },
            serde_json::json!({
                "plan": [
                    { "step": "Pending task", "status": "pending" },
                    { "step": "In progress task", "status": "in_progress" },
                    { "step": "Blocked task", "status": "blocked" },
                    { "step": "Completed task", "status": "completed" },
                    { "step": "Failed task", "status": "failed" }
                ]
            }),
        )
        .await
        .expect("should accept all five task statuses");

    assert!(!result.is_error);
    let tasks = result.output["plan"]["tasks"].as_array().unwrap();
    assert_eq!(tasks[0]["status"], "pending");
    assert_eq!(tasks[1]["status"], "in_progress");
    assert_eq!(tasks[2]["status"], "blocked");
    assert_eq!(tasks[3]["status"], "completed");
    assert_eq!(tasks[4]["status"], "failed");
}

#[tokio::test]
async fn update_plan_accepts_optional_queue_todos() {
    use agent_tool::{Tool, ToolContext};
    let tool = agent_tool::builtin::update_plan::UpdatePlanTool;
    let result = tool
        .execute(
            ToolContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
            },
            serde_json::json!({
                "plan": [
                    { "step": "Plan step 1", "status": "completed" }
                ],
                "queue": {
                    "todos": [
                        { "id": "todo-1", "title": "TODO Item 1", "status": "pending" },
                        { "id": "todo-2", "title": "TODO Item 2", "status": "in_progress" }
                    ]
                }
            }),
        )
        .await
        .expect("should accept optional queue.todos");

    assert!(!result.is_error);
    assert!(result.output["plan"]["queue"]["todos"].is_array());
    let todos = result.output["plan"]["queue"]["todos"].as_array().unwrap();
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0]["title"], "TODO Item 1");
    assert_eq!(todos[1]["status"], "in_progress");
}

#[tokio::test]
async fn update_plan_rejects_invalid_queue_todo_status() {
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
                    { "step": "Plan step", "status": "pending" }
                ],
                "queue": {
                    "todos": [
                        { "id": "todo-1", "title": "TODO", "status": "invalid_status" }
                    ]
                }
            }),
        )
        .await
        .expect_err("should reject invalid queue todo status");

    assert!(err.to_string().contains("invalid") || err.to_string().contains("status"));
}

#[tokio::test]
async fn update_plan_infers_progress_and_lifecycle_when_absent() {
    use agent_tool::{Tool, ToolContext};
    let tool = agent_tool::builtin::update_plan::UpdatePlanTool;
    let result = tool
        .execute(
            ToolContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
            },
            serde_json::json!({
                "plan": [
                    { "step": "Step 1", "status": "completed" },
                    { "step": "Step 2", "status": "in_progress" }
                ]
            }),
        )
        .await
        .expect("should infer progress and lifecycle");

    assert!(!result.is_error);
    // Should infer progress from tasks
    assert!(result.output["plan"]["progress"].is_object());
    // Should infer lifecycle_status from tasks
    assert!(result.output["plan"]["lifecycle_status"].is_string());
}
