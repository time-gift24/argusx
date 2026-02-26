#[test]
fn test_chat_request_serialization() {
    let request =
        bigmodel_api::ChatRequest::new("glm-4", vec![bigmodel_api::Message::user("Hello")])
            .temperature(0.7)
            .max_tokens(1000);

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("glm-4"));
    assert!(json.contains("Hello"));
}

#[test]
fn test_tool_serialization_uses_function_lowercase_type() {
    let request =
        bigmodel_api::ChatRequest::new("glm-4", vec![bigmodel_api::Message::user("Hello")]).tools(
            vec![bigmodel_api::Tool::Function(bigmodel_api::FunctionTool {
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
            })],
        );

    let value = serde_json::to_value(&request).expect("serialize request");
    assert_eq!(value["tools"][0]["type"], "function");
}

#[test]
fn test_tool_choice_serializes_to_auto_string() {
    let request =
        bigmodel_api::ChatRequest::new("glm-5", vec![bigmodel_api::Message::user("Hello")])
            .tool_choice(bigmodel_api::ToolChoice::Auto);

    let value = serde_json::to_value(&request).expect("serialize request");
    assert_eq!(value["tool_choice"], "auto");
}

#[test]
fn test_retrieval_tool_serialization_matches_schema_shape() {
    let request =
        bigmodel_api::ChatRequest::new("glm-5", vec![bigmodel_api::Message::user("Find docs")])
            .tools(vec![bigmodel_api::Tool::Retrieval(
                bigmodel_api::RetrievalTool {
                    retrieval: bigmodel_api::RetrievalObject {
                        knowledge_id: "kb-123".to_string(),
                        prompt_template: None,
                    },
                },
            )]);

    let value = serde_json::to_value(&request).expect("serialize request");
    assert_eq!(value["tools"][0]["type"], "retrieval");
    assert_eq!(value["tools"][0]["retrieval"]["knowledge_id"], "kb-123");
}

#[test]
fn test_multimodal_image_part_uses_discriminated_union_shape() {
    let request = bigmodel_api::ChatRequest::new(
        "glm-4.6v",
        vec![bigmodel_api::Message {
            role: bigmodel_api::Role::User,
            content: bigmodel_api::Content::Multimodal(vec![
                bigmodel_api::ContentPart::ImageUrl {
                    image_url: bigmodel_api::ImageUrl {
                        url: "https://example.com/a.png".to_string(),
                    },
                },
                bigmodel_api::ContentPart::Text {
                    text: "describe this image".to_string(),
                },
            ]),
            reasoning_content: None,
        }],
    );

    let value = serde_json::to_value(&request).expect("serialize request");
    assert_eq!(value["messages"][0]["content"][0]["type"], "image_url");
    assert_eq!(
        value["messages"][0]["content"][0]["image_url"]["url"],
        "https://example.com/a.png"
    );
    assert_eq!(value["messages"][0]["content"][1]["type"], "text");
}
