use provider::dialect::openai::schema::common::{ReasoningEffort, Role, Verbosity};
use provider::dialect::openai::schema::request::ChatCompletionsOptions;
use serde_json::json;

#[test]
fn deserializes_max_tokens_alias_into_max_completion_tokens() {
    let value = json!({
        "model": "glm-5",
        "messages": [],
        "max_tokens": 1024
    });

    let options: ChatCompletionsOptions = serde_json::from_value(value).unwrap();
    assert_eq!(options.max_completion_tokens, Some(1024));
}

#[test]
fn stream_true_defaults_include_usage() {
    let mut options = ChatCompletionsOptions {
        model: "glm-5".to_string(),
        messages: Vec::new(),
        stream: Some(true),
        ..Default::default()
    };

    options.apply_stream_defaults();

    assert_eq!(
        options
            .stream_options
            .as_ref()
            .and_then(|v| v.include_usage),
        Some(true)
    );
}

#[test]
fn legacy_json_serializer_emits_max_tokens() {
    let options = ChatCompletionsOptions {
        model: "glm-5".to_string(),
        messages: Vec::new(),
        max_completion_tokens: Some(2048),
        stream: Some(true),
        ..Default::default()
    };

    let value = options.to_legacy_json().unwrap();
    let obj = value.as_object().unwrap();

    assert_eq!(obj.get("max_tokens"), Some(&json!(2048)));
    assert!(!obj.contains_key("max_completion_tokens"));
    assert_eq!(
        obj.get("stream_options")
            .and_then(|v| v.get("include_usage")),
        Some(&json!(true))
    );
}

#[test]
fn preserves_unknown_role_reasoning_and_verbosity_values() {
    let value = json!({
        "model": "glm-5",
        "messages": [{ "role": "qa_agent", "content": "hello" }],
        "reasoning_effort": "ultra",
        "verbosity": "verbose_plus"
    });

    let options: ChatCompletionsOptions = serde_json::from_value(value).unwrap();

    assert!(matches!(
        options.messages[0].role,
        Role::Unknown(ref v) if v == "qa_agent"
    ));
    assert!(matches!(
        options.reasoning_effort,
        Some(ReasoningEffort::Unknown(ref v)) if v == "ultra"
    ));
    assert!(matches!(
        options.verbosity,
        Some(Verbosity::Unknown(ref v)) if v == "verbose_plus"
    ));
}
