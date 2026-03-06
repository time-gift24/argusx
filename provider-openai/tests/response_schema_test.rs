use provider_openai::schema::response::ChatCompletionsResponse;
use serde_json::json;

#[test]
fn parses_response_with_usage_details_and_logprobs() {
    let value = json!({
        "id": "resp_1",
        "object": "chat.completion",
        "created": 1772761165u64,
        "model": "glm-5",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "hello"
            },
            "logprobs": {
                "content": [{
                    "token": "hello",
                    "logprob": -0.1,
                    "bytes": [104,101,108,108,111],
                    "top_logprobs": [{
                        "token": "hello",
                        "logprob": -0.1,
                        "bytes": [104,101,108,108,111]
                    }]
                }]
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 7,
            "total_tokens": 17,
            "prompt_tokens_details": {
                "cached_tokens": 3
            },
            "completion_tokens_details": {
                "reasoning_tokens": 2
            }
        },
        "system_fingerprint": "fp_123",
        "service_tier": "default"
    });

    let resp: ChatCompletionsResponse = serde_json::from_value(value).unwrap();
    assert_eq!(resp.id, "resp_1");
    assert_eq!(resp.created, 1772761165);
    assert_eq!(resp.choices.len(), 1);
    assert_eq!(resp.choices[0].message.content.as_deref(), Some("hello"));
    assert_eq!(resp.usage.as_ref().unwrap().total_tokens, 17);
    assert_eq!(
        resp.usage
            .as_ref()
            .unwrap()
            .prompt_tokens_details
            .as_ref()
            .unwrap()
            .cached_tokens,
        3
    );
}

#[test]
fn tolerates_unknown_response_fields_via_flatten_extra() {
    let value = json!({
        "id": "resp_2",
        "object": "chat.completion",
        "created": 1772761165u64,
        "model": "glm-5",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "ok",
                "custom_message_field": "x"
            },
            "finish_reason": null,
            "custom_choice_field": 1
        }],
        "custom_response_field": "y"
    });

    let resp: ChatCompletionsResponse = serde_json::from_value(value).unwrap();
    assert_eq!(resp.choices[0].message.content.as_deref(), Some("ok"));
    assert_eq!(resp.extra.get("custom_response_field"), Some(&json!("y")));
}
