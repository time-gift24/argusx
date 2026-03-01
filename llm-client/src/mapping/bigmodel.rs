// llm-client/src/mapping/bigmodel.rs
use bigmodel_api::{ChatRequest, ChatResponse, ChatResponseChunk, FunctionDefinition, FunctionTool, Message, Role, Tool as BigModelTool};
use crate::{LlmChunk, LlmRequest, LlmResponse, LlmRole, LlmToolCall, LlmUsage};

/// Convert our generic LlmRequest to BigModel's ChatRequest.
pub fn to_bigmodel_request(req: &LlmRequest) -> ChatRequest {
    let messages: Vec<Message> = req.messages.iter().map(|m| {
        let role = match m.role {
            LlmRole::System => Role::System,
            LlmRole::User => Role::User,
            LlmRole::Assistant => Role::Assistant,
            LlmRole::Tool => Role::Tool,
        };
        Message {
            role,
            content: m.content.clone().into(),
            reasoning_content: None,
        }
    }).collect();

    // Map generic tools to BigModel function tools
    let tools = req.tools.as_ref().map(|tools| {
        tools.iter().map(|t| {
            BigModelTool::Function(FunctionTool {
                function: FunctionDefinition {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            })
        }).collect()
    });

    ChatRequest {
        model: req.model.clone(),
        messages,
        do_sample: None,
        temperature: req.temperature,
        top_p: req.top_p,
        max_tokens: req.max_tokens,
        stream: req.stream,
        tool_stream: None,
        tools,
        tool_choice: None,
        stop: None,
        response_format: None,
        request_id: None,
        user_id: None,
        thinking: None,
    }
}

/// Convert BigModel's ChatResponse to our generic LlmResponse.
pub fn to_llm_response(resp: &ChatResponse) -> LlmResponse {
    let output_text = resp.choices.first()
        .and_then(|c| {
            match &c.message.content {
                bigmodel_api::Content::Text(s) => Some(s.clone()),
                _ => None,
            }
        })
        .unwrap_or_default();

    let finish_reason = resp.choices.first()
        .as_ref()
        .map(|c| c.finish_reason.clone());

    let usage = resp.usage.as_ref().map(|u| LlmUsage {
        input_tokens: u.prompt_tokens.try_into().unwrap_or(0),
        output_tokens: u.completion_tokens.try_into().unwrap_or(0),
        total_tokens: u.total_tokens.try_into().unwrap_or(0),
    });

    // Map additional extensions (web_search, video_result, content_filter)
    let extensions = serde_json::json!({
        "web_search": resp.web_search,
        "video_result": resp.video_result,
        "content_filter": resp.content_filter,
    });

    LlmResponse {
        id: resp.id.clone(),
        created: resp.created,
        model: resp.model.clone(),
        output_text,
        finish_reason,
        request_id: resp.request_id.clone(),
        usage,
        extensions,
    }
}

/// Convert BigModel's ChatResponseChunk to our generic LlmChunk.
pub fn to_llm_chunk(chunk: ChatResponseChunk) -> LlmChunk {
    let delta = chunk.choices.first().map(|c| &c.delta);

    let delta_text = delta.and_then(|d| d.content.clone());
    let delta_reasoning = delta.and_then(|d| d.reasoning_content.clone());
    let finish_reason = chunk.choices.first().and_then(|c| c.finish_reason.clone());

    // Map BigModel's tool_calls to generic LlmToolCall
    let delta_tool_calls = delta.and_then(|d| {
        d.tool_calls.as_ref().map(|calls| {
            calls.iter().map(|tc| {
                LlmToolCall {
                    call_id: tc.id.clone(),
                    tool_name: tc.function.as_ref().and_then(|f| f.name.clone()),
                    arguments: tc.function.as_ref().and_then(|f| f.arguments.clone()),
                }
            }).collect()
        })
    });

    let usage = chunk.usage.as_ref().map(|u| LlmUsage {
        input_tokens: u.prompt_tokens.try_into().unwrap_or(0),
        output_tokens: u.completion_tokens.try_into().unwrap_or(0),
        total_tokens: u.total_tokens.try_into().unwrap_or(0),
    });

    LlmChunk {
        id: chunk.id,
        created: chunk.created,
        model: chunk.model,
        delta_text,
        delta_reasoning,
        delta_tool_calls,
        finish_reason,
        usage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LlmRequest, LlmRole, LlmTool, LlmToolCall};
    use bigmodel_api::{ChatResponse, ChatResponseChunk, Choice, ChoiceChunk, Delta, Usage};

    /// Test that tools are mapped to BigModel format
    #[test]
    fn to_bigmodel_request_maps_tools() {
        let tools = vec![LlmTool {
            name: "echo".to_string(),
            description: "Echo back the input".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                }
            }),
        }];

        let req = LlmRequest {
            model: "glm-5".to_string(),
            messages: vec![crate::LlmMessage {
                role: LlmRole::User,
                content: "hello".to_string(),
            }],
            stream: true,
            max_tokens: Some(128),
            temperature: Some(0.7),
            top_p: Some(0.9),
            tools: Some(tools),
        };

        let bigmodel_req = to_bigmodel_request(&req);

        // Tools should be mapped, not None
        assert!(bigmodel_req.tools.is_some(), "tools should be mapped");
        let tools = bigmodel_req.tools.unwrap();
        assert_eq!(tools.len(), 1);
        // Check it's a function tool
        assert!(matches!(tools[0], bigmodel_api::Tool::Function(_)));
    }

    /// Test that tool_calls in BigModel delta are mapped to LlmChunk
    #[test]
    fn to_llm_chunk_maps_tool_calls() {
        use bigmodel_api::{ChoiceChunk, Delta, DeltaToolCall, DeltaToolFunction};

        let chunk = bigmodel_api::ChatResponseChunk {
            id: "test-id".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![DeltaToolCall {
                        id: Some("call-123".to_string()),
                        type_field: Some("function".to_string()),
                        function: Some(DeltaToolFunction {
                            name: Some("echo".to_string()),
                            arguments: Some(r#"{"text":"hello"}"#.to_string()),
                        }),
                        index: Some(0),
                    }]),
                },
                finish_reason: None,
            }],
            usage: None,
        };

        let llm_chunk = to_llm_chunk(chunk);

        assert!(llm_chunk.delta_tool_calls.is_some(), "tool_calls should be mapped");
        let calls = llm_chunk.delta_tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].call_id.as_deref(), Some("call-123"));
        assert_eq!(calls[0].tool_name.as_deref(), Some("echo"));
        assert_eq!(calls[0].arguments.as_deref(), Some(r#"{"text":"hello"}"#));
    }

    /// Test that negative prompt_tokens maps to 0 (not wrapped value)
    #[test]
    fn to_llm_response_negative_prompt_tokens_maps_to_zero() {
        use bigmodel_api::Choice;

        let resp = ChatResponse {
            id: "test".to_string(),
            request_id: None,
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![Choice {
                index: 0,
                message: bigmodel_api::Message {
                    role: bigmodel_api::Role::Assistant,
                    content: bigmodel_api::Content::Text("hello".to_string()),
                    reasoning_content: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Some(Usage {
                prompt_tokens: -10,
                completion_tokens: 100,
                total_tokens: 90,
            }),
            web_search: vec![],
            video_result: vec![],
            content_filter: vec![],
        };

        let llm_resp = to_llm_response(&resp);

        assert_eq!(llm_resp.usage.as_ref().unwrap().input_tokens, 0);
    }

    /// Test that negative completion_tokens maps to 0 (not wrapped value)
    #[test]
    fn to_llm_response_negative_completion_tokens_maps_to_zero() {
        use bigmodel_api::Choice;

        let resp = ChatResponse {
            id: "test".to_string(),
            request_id: None,
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![Choice {
                index: 0,
                message: bigmodel_api::Message {
                    role: bigmodel_api::Role::Assistant,
                    content: bigmodel_api::Content::Text("hello".to_string()),
                    reasoning_content: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: -5,
                total_tokens: 95,
            }),
            web_search: vec![],
            video_result: vec![],
            content_filter: vec![],
        };

        let llm_resp = to_llm_response(&resp);

        assert_eq!(llm_resp.usage.as_ref().unwrap().output_tokens, 0);
    }

    /// Test that negative total_tokens maps to 0 (not wrapped value)
    #[test]
    fn to_llm_response_negative_total_tokens_maps_to_zero() {
        use bigmodel_api::Choice;

        let resp = ChatResponse {
            id: "test".to_string(),
            request_id: None,
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![Choice {
                index: 0,
                message: bigmodel_api::Message {
                    role: bigmodel_api::Role::Assistant,
                    content: bigmodel_api::Content::Text("hello".to_string()),
                    reasoning_content: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: -150,
            }),
            web_search: vec![],
            video_result: vec![],
            content_filter: vec![],
        };

        let llm_resp = to_llm_response(&resp);

        assert_eq!(llm_resp.usage.as_ref().unwrap().total_tokens, 0);
    }

    /// Test that negative prompt_tokens in chunk maps to 0
    #[test]
    fn to_llm_chunk_negative_prompt_tokens_maps_to_zero() {
        let chunk = bigmodel_api::ChatResponseChunk {
            id: "test-id".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: Some("hello".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: -10,
                completion_tokens: 100,
                total_tokens: 90,
            }),
        };

        let llm_chunk = to_llm_chunk(chunk);

        assert_eq!(llm_chunk.usage.as_ref().unwrap().input_tokens, 0);
    }

    /// Test that negative completion_tokens in chunk maps to 0
    #[test]
    fn to_llm_chunk_negative_completion_tokens_maps_to_zero() {
        let chunk = bigmodel_api::ChatResponseChunk {
            id: "test-id".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: Some("hello".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: -5,
                total_tokens: 95,
            }),
        };

        let llm_chunk = to_llm_chunk(chunk);

        assert_eq!(llm_chunk.usage.as_ref().unwrap().output_tokens, 0);
    }

    /// Test that negative total_tokens in chunk maps to 0
    #[test]
    fn to_llm_chunk_negative_total_tokens_maps_to_zero() {
        let chunk = bigmodel_api::ChatResponseChunk {
            id: "test-id".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: Some("hello".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: -150,
            }),
        };

        let llm_chunk = to_llm_chunk(chunk);

        assert_eq!(llm_chunk.usage.as_ref().unwrap().total_tokens, 0);
    }
}
