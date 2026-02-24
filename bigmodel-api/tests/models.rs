#[test]
fn test_chat_request_serialization() {
    let request = bigmodel_api::ChatRequest {
        model: "glm-4".to_string(),
        messages: vec![bigmodel_api::Message {
            role: bigmodel_api::Role::User,
            content: bigmodel_api::Content::Text("Hello".to_string()),
            reasoning_content: None,
        }],
        temperature: Some(0.7),
        top_p: None,
        max_tokens: Some(1000),
        stream: false,
        tools: None,
        tool_choice: None,
        thinking: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("glm-4"));
    assert!(json.contains("Hello"));
}

#[test]
fn test_tool_serialization_uses_function_lowercase_type() {
    let request = bigmodel_api::ChatRequest::new(
        "glm-4",
        vec![bigmodel_api::Message::user("Hello")],
    )
    .tools(vec![bigmodel_api::Tool::Function(
        bigmodel_api::FunctionTool {
            function: bigmodel_api::FunctionDefinition {
                name: "shell".to_string(),
                description: "Execute shell command".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" }
                    },
                    "required": ["command"]
                }),
            },
        },
    )]);

    let value = serde_json::to_value(&request).expect("serialize request");
    assert_eq!(value["tools"][0]["type"], "function");
}
