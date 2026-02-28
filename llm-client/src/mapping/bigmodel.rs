// llm-client/src/mapping/bigmodel.rs
use bigmodel_api::{ChatRequest, ChatResponse, ChatResponseChunk, Message, Role};
use crate::{LlmChunk, LlmRequest, LlmResponse, LlmRole, LlmUsage};

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

    ChatRequest {
        model: req.model.clone(),
        messages,
        do_sample: None,
        temperature: req.temperature,
        top_p: req.top_p,
        max_tokens: req.max_tokens,
        stream: req.stream,
        tool_stream: None,
        tools: None,
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

    let usage = resp.usage.as_ref().map(|u| LlmUsage {
        input_tokens: u.prompt_tokens as u64,
        output_tokens: u.completion_tokens as u64,
        total_tokens: u.total_tokens as u64,
    });

    LlmResponse {
        id: resp.id.clone(),
        model: resp.model.clone(),
        output_text,
        usage,
    }
}

/// Convert BigModel's ChatResponseChunk to our generic LlmChunk.
pub fn to_llm_chunk(chunk: ChatResponseChunk) -> LlmChunk {
    let delta = chunk.choices.first().map(|c| &c.delta);

    let delta_text = delta.and_then(|d| d.content.clone());
    let delta_reasoning = delta.and_then(|d| d.reasoning_content.clone());
    let finish_reason = chunk.choices.first().and_then(|c| c.finish_reason.clone());

    let usage = chunk.usage.as_ref().map(|u| LlmUsage {
        input_tokens: u.prompt_tokens as u64,
        output_tokens: u.completion_tokens as u64,
        total_tokens: u.total_tokens as u64,
    });

    LlmChunk {
        delta_text,
        delta_reasoning,
        finish_reason,
        usage,
    }
}
