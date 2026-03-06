use argus_core::{Meta, ResponseEvent, ToolCall, Usage};
use crate::chunk::{ChatCompletionsChunk, ToolCallChunk, ChunkUsage};
use crate::parser::parse_chunk;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error: {0}")]
    Parse(#[from] crate::parser::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug)]
struct PendingToolCall {
    sequence: u32,
    call_id: String,
    name: String,
    arguments_json: String,
}

pub struct Mapper {
    provider: String,
    created: bool,
    terminated: bool,
    tool_calls: HashMap<String, PendingToolCall>,
    usage: Option<Usage>,
}

impl Mapper {
    pub fn new(provider: String) -> Self {
        Self {
            provider,
            created: false,
            terminated: false,
            tool_calls: HashMap::new(),
            usage: None,
        }
    }

    pub fn feed(&mut self, raw: &str) -> Result<Vec<ResponseEvent>, Error> {
        if self.terminated {
            return Err(Error::Protocol("event after terminal".into()));
        }
        let chunk: ChatCompletionsChunk = parse_chunk(raw)?;

        let mut events = Vec::new();

        // Emit Created only once
        if !self.created {
            events.push(ResponseEvent::Created(Meta {
                id: chunk.id,
                created: chunk.created,
                object: chunk.object_type,
                model: chunk.model.clone(),
            }));
            self.created = true;
        }

        for choice in &chunk.choices {
            // Handle content delta
            if let Some(content) = &choice.delta.content {
                if !content.is_empty() {
                    events.push(ResponseEvent::ContentDelta(content.clone().into()));
                }
            }

            // Handle reasoning delta
            if let Some(reasoning) = &choice.delta.reasoning_content {
                if !reasoning.is_empty() {
                    events.push(ResponseEvent::ReasoningDelta(reasoning.clone().into()));
                }
            }

            // Handle tool calls
            if let Some(tool_calls) = &choice.delta.tool_calls {
                for tc in tool_calls {
                    self.process_tool_call_chunk(tc, &mut events);
                }
            }

            // Handle finish_reason
            if let Some(finish_reason) = &choice.finish_reason {
                if finish_reason == "tool_calls" {
                    // Emit all accumulated tool calls as ToolDone
                    for (_, tc) in self.tool_calls.drain() {
                        events.push(ResponseEvent::ToolDone(ToolCall::FunctionCall {
                            sequence: tc.sequence,
                            call_id: tc.call_id,
                            name: tc.name,
                            arguments_json: tc.arguments_json,
                        }));
                    }
                }
            }
        }

        // Capture usage when present
        if let Some(chunk_usage) = chunk.usage {
            self.usage = Some(Usage {
                input_tokens: chunk_usage.prompt_tokens,
                output_tokens: chunk_usage.completion_tokens,
                total_tokens: chunk_usage.total_tokens,
            });
        }

        Ok(events)
    }

    fn process_tool_call_chunk(&mut self, tc: &ToolCallChunk, events: &mut Vec<ResponseEvent>) {
        let call_id = tc.id.as_ref().cloned().unwrap_or_default();
        let sequence = tc.index.unwrap_or(0);

        // Get or create pending tool call
        let pending = self.tool_calls.entry(call_id.clone()).or_insert(PendingToolCall {
            sequence,
            call_id: call_id.clone(),
            name: String::new(),
            arguments_json: String::new(),
        });

        // Update name if present
        if let Some(name) = &tc.function.name {
            pending.name = name.clone();
        }

        // Accumulate arguments
        if let Some(args) = &tc.function.arguments {
            pending.arguments_json.push_str(args);
            // Emit ToolDelta for incremental arguments
            events.push(ResponseEvent::ToolDelta(args.clone().into()));
        }
    }

    pub fn on_done(&mut self) -> Result<Vec<ResponseEvent>, Error> {
        if self.terminated {
            return Err(Error::Protocol("already terminated".into()));
        }
        self.terminated = true;
        Ok(vec![ResponseEvent::Done(self.usage.take())])
    }
}
