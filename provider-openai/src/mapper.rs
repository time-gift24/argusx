use crate::chunk::ToolCallChunk;
use crate::parser::parse_chunk;
use argus_core::{Meta, ResponseEvent, ToolCall, Usage};
use std::collections::BTreeMap;
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
    call_id: Option<String>,
    name: Option<String>,
    arguments_json: String,
}

pub struct Mapper {
    created: bool,
    terminated: bool,
    tool_calls: BTreeMap<u32, PendingToolCall>,
    usage: Option<Usage>,
    content_buffer: String,
    reasoning_buffer: String,
    content_done: bool,
    reasoning_done: bool,
}

impl Mapper {
    pub fn new(_provider: String) -> Self {
        Self {
            created: false,
            terminated: false,
            tool_calls: BTreeMap::new(),
            usage: None,
            content_buffer: String::new(),
            reasoning_buffer: String::new(),
            content_done: false,
            reasoning_done: false,
        }
    }

    pub fn feed(&mut self, raw: &str) -> Result<Vec<ResponseEvent>, Error> {
        if self.terminated {
            return Err(Error::Protocol("event after terminal".into()));
        }
        let chunk = parse_chunk(raw)?;

        let mut events = Vec::new();

        // Emit Created only once
        if !self.created {
            events.push(ResponseEvent::Created(Meta {
                id: chunk.id.clone(),
                created: chunk.created,
                object: chunk.object_type.clone(),
                model: chunk.model.clone(),
            }));
            self.created = true;
        }

        for choice in &chunk.choices {
            // Handle content delta
            if let Some(content) = &choice.delta.content {
                if !content.is_empty() {
                    self.content_buffer.push_str(content);
                    events.push(ResponseEvent::ContentDelta(content.clone().into()));
                }
            }

            // Handle reasoning delta
            if let Some(reasoning) = &choice.delta.reasoning_content {
                if !reasoning.is_empty() {
                    self.reasoning_buffer.push_str(reasoning);
                    events.push(ResponseEvent::ReasoningDelta(reasoning.clone().into()));
                }
            }

            // Handle tool calls
            if let Some(tool_calls) = &choice.delta.tool_calls {
                for tc in tool_calls {
                    self.process_tool_call_chunk(tc, &mut events)?;
                }
            }

            // Handle finish_reason
            if let Some(finish_reason) = &choice.finish_reason {
                match finish_reason.as_str() {
                    "stop" => {
                        self.emit_text_done_events(&mut events);
                    }
                    "tool_calls" => {
                        self.flush_tool_calls(&mut events)?;
                        self.emit_text_done_events(&mut events);
                    }
                    _ => {}
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

    fn process_tool_call_chunk(
        &mut self,
        tc: &ToolCallChunk,
        events: &mut Vec<ResponseEvent>,
    ) -> Result<(), Error> {
        let sequence = tc
            .index
            .ok_or_else(|| Error::Protocol("missing tool call index".into()))?;

        // Get or create pending tool call
        let pending = self
            .tool_calls
            .entry(sequence)
            .or_insert_with(|| PendingToolCall {
                sequence,
                call_id: None,
                name: None,
                arguments_json: String::new(),
            });

        // Keep call_id stable by sequence
        if let Some(call_id) = &tc.id {
            if call_id.is_empty() {
                return Err(Error::Protocol(format!(
                    "empty call_id for tool call sequence {sequence}"
                )));
            }
            match &pending.call_id {
                Some(existing) if existing != call_id => {
                    return Err(Error::Protocol(format!(
                        "conflicting call_id for tool call sequence {sequence}: '{existing}' vs '{call_id}'"
                    )));
                }
                _ => pending.call_id = Some(call_id.clone()),
            }
        }

        // Update name if present
        if let Some(name) = &tc.function.name {
            if !name.is_empty() {
                match &pending.name {
                    Some(existing) if existing != name => {
                        return Err(Error::Protocol(format!(
                            "conflicting tool name for sequence {sequence}: '{existing}' vs '{name}'"
                        )));
                    }
                    _ => pending.name = Some(name.clone()),
                }
            }
        }

        // Accumulate arguments
        if let Some(args) = &tc.function.arguments {
            if !args.is_empty() {
                pending.arguments_json.push_str(args);
                // Emit ToolDelta for incremental arguments
                events.push(ResponseEvent::ToolDelta(args.clone().into()));
            }
        }

        Ok(())
    }

    fn flush_tool_calls(&mut self, events: &mut Vec<ResponseEvent>) -> Result<(), Error> {
        for (_sequence, tc) in std::mem::take(&mut self.tool_calls) {
            let call_id = tc.call_id.ok_or_else(|| {
                Error::Protocol(format!(
                    "missing call_id for tool call sequence {}",
                    tc.sequence
                ))
            })?;
            let name = tc.name.ok_or_else(|| {
                Error::Protocol(format!("missing tool name for tool call '{}'", call_id))
            })?;

            events.push(ResponseEvent::ToolDone(ToolCall::FunctionCall {
                sequence: tc.sequence,
                call_id,
                name,
                arguments_json: tc.arguments_json,
            }));
        }
        Ok(())
    }

    fn emit_text_done_events(&mut self, events: &mut Vec<ResponseEvent>) {
        if !self.reasoning_done && !self.reasoning_buffer.is_empty() {
            self.reasoning_done = true;
            events.push(ResponseEvent::ReasoningDone(std::mem::take(
                &mut self.reasoning_buffer,
            )));
        }
        if !self.content_done && !self.content_buffer.is_empty() {
            self.content_done = true;
            events.push(ResponseEvent::ContentDone(std::mem::take(
                &mut self.content_buffer,
            )));
        }
    }

    pub fn on_done(&mut self) -> Result<Vec<ResponseEvent>, Error> {
        if self.terminated {
            return Err(Error::Protocol("already terminated".into()));
        }
        self.terminated = true;

        let mut events = Vec::new();
        self.flush_tool_calls(&mut events)?;
        self.emit_text_done_events(&mut events);
        events.push(ResponseEvent::Done(self.usage.take()));
        Ok(events)
    }
}
