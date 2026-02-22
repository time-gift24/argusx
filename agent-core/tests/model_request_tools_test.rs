use agent_core::tools::{ToolExecutionPolicy, ToolSpec};
use agent_core::{InputEnvelope, ModelRequest};

#[test]
fn model_request_serializes_tools() {
    let req = ModelRequest {
        epoch: 0,
        transcript: vec![],
        inputs: vec![InputEnvelope::user_text("hi")],
        tools: vec![ToolSpec {
            name: "echo".to_string(),
            description: "echo args".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            execution_policy: ToolExecutionPolicy::default(),
        }],
    };
    let raw = serde_json::to_string(&req).unwrap();
    assert!(raw.contains("\"tools\""));
}
