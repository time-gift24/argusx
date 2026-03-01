use agent_core::tools::{ToolExecutionPolicy, ToolSpec};
use agent_core::{InputEnvelope, ModelRequest};

#[test]
fn model_request_serializes_tools() {
    let req = ModelRequest {
        epoch: 0,
        provider: "bigmodel".to_string(),
        model: "glm-5".to_string(),
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
    assert!(raw.contains("\"provider\":\"bigmodel\""));
    assert!(raw.contains("\"model\":\"glm-5\""));
}
