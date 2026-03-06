use crate::dialect::openai::parser::parse_payload;
use crate::dialect::openai::schema::stream::{
    ChatCompletionsStreamChunk, ChatCompletionsStreamEvent, DeltaToolCall,
};
use crate::normalize::tool_calls::{classify_tool_call, parse_zai_mcp_json, ToolCallKind};
use argus_core::{BuiltinToolCall, McpCall, Meta, ResponseEvent, ToolCall, Usage};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const INITIAL_TEXT_BUFFER_CAPACITY: usize = 256;
const INITIAL_TOOL_ARGS_BUFFER_CAPACITY: usize = 128;
const TYPICAL_EVENTS_PER_CHUNK: usize = 4;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error: {0}")]
    Parse(#[from] crate::dialect::openai::parser::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug)]
struct PendingToolCall {
    call_type: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments_json: String,
}

pub struct Mapper {
    created: bool,
    terminated: bool,
    tool_calls: BTreeMap<u32, PendingToolCall>,
    emitted_tool_sequences: BTreeSet<u32>,
    usage: Option<Usage>,
    content_buffer: String,
    reasoning_buffer: String,
    content_done: bool,
    reasoning_done: bool,
}

impl Mapper {
    pub fn new() -> Self {
        Self {
            created: false,
            terminated: false,
            tool_calls: BTreeMap::new(),
            emitted_tool_sequences: BTreeSet::new(),
            usage: None,
            content_buffer: String::with_capacity(INITIAL_TEXT_BUFFER_CAPACITY),
            reasoning_buffer: String::with_capacity(INITIAL_TEXT_BUFFER_CAPACITY),
            content_done: false,
            reasoning_done: false,
        }
    }

    pub fn feed(&mut self, raw: &str) -> Result<Vec<ResponseEvent>, Error> {
        if self.terminated {
            return Err(Error::Protocol("event after terminal".into()));
        }

        match parse_payload(raw)? {
            ChatCompletionsStreamEvent::Chunk(chunk) => self.process_chunk(chunk),
            ChatCompletionsStreamEvent::Open => Ok(Vec::new()),
            ChatCompletionsStreamEvent::Done => Err(Error::Protocol(
                "received [DONE] in feed(); call on_done() instead".into(),
            )),
            ChatCompletionsStreamEvent::Error(err) => Err(Error::Protocol(format!(
                "upstream stream error: {}",
                err.message()
            ))),
        }
    }

    fn process_chunk(
        &mut self,
        chunk: ChatCompletionsStreamChunk,
    ) -> Result<Vec<ResponseEvent>, Error> {
        let ChatCompletionsStreamChunk {
            id,
            object,
            created,
            model,
            choices,
            usage,
            ..
        } = chunk;
        let mut events = Vec::with_capacity(TYPICAL_EVENTS_PER_CHUNK);

        if !self.created {
            let created = i64::try_from(created).map_err(|_| {
                Error::Protocol(format!("created is out of i64 range: {}", created))
            })?;
            events.push(ResponseEvent::Created(Meta {
                id,
                created,
                object,
                model,
            }));
            self.created = true;
        }

        for mut choice in choices {
            if let Some(content) = choice.delta.content.take()
                && !content.is_empty()
            {
                self.content_buffer.push_str(&content);
                events.push(ResponseEvent::ContentDelta(content.into()));
            }

            if let Some(reasoning) = choice.delta.reasoning_content.take()
                && !reasoning.is_empty()
            {
                self.reasoning_buffer.push_str(&reasoning);
                events.push(ResponseEvent::ReasoningDelta(reasoning.into()));
            }

            if let Some(tool_calls) = choice.delta.tool_calls.take() {
                for tc in tool_calls {
                    self.process_tool_call_chunk(tc, &mut events)?;
                }
            }

            match choice.finish_reason.as_deref() {
                Some("stop") => self.emit_text_done_events(&mut events),
                Some("tool_calls") => {
                    self.flush_tool_calls(&mut events)?;
                    self.emit_text_done_events(&mut events);
                }
                _ => {}
            }
        }

        if let Some(chunk_usage) = usage {
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
        tc: DeltaToolCall,
        events: &mut Vec<ResponseEvent>,
    ) -> Result<(), Error> {
        let sequence = tc
            .index
            .ok_or_else(|| Error::Protocol("missing tool call index".into()))?;
        if self.emitted_tool_sequences.contains(&sequence) {
            return Ok(());
        }

        let pending = self
            .tool_calls
            .entry(sequence)
            .or_insert_with(|| PendingToolCall {
                call_type: None,
                call_id: None,
                name: None,
                arguments_json: String::with_capacity(INITIAL_TOOL_ARGS_BUFFER_CAPACITY),
            });

        if let Some(call_type) = tc.type_
            && !call_type.is_empty()
        {
            match pending.call_type.as_ref() {
                Some(existing) if existing != &call_type => {
                    return Err(Error::Protocol(format!(
                        "conflicting tool type for sequence {sequence}: '{existing}' vs '{call_type}'"
                    )));
                }
                _ => pending.call_type = Some(call_type),
            }
        }

        if let Some(call_id) = tc.id {
            if call_id.is_empty() {
                return Err(Error::Protocol(format!(
                    "empty call_id for tool call sequence {sequence}"
                )));
            }
            match pending.call_id.as_ref() {
                Some(existing) if existing != &call_id => {
                    return Err(Error::Protocol(format!(
                        "conflicting call_id for tool call sequence {sequence}: '{existing}' vs '{call_id}'"
                    )));
                }
                _ => pending.call_id = Some(call_id),
            }
        }

        if let Some(function) = tc.function {
            if let Some(name) = function.name
                && !name.is_empty()
            {
                match pending.name.as_ref() {
                    Some(existing) if existing != &name => {
                        return Err(Error::Protocol(format!(
                            "conflicting tool name for sequence {sequence}: '{existing}' vs '{name}'"
                        )));
                    }
                    _ => pending.name = Some(name),
                }
            }

            if let Some(args) = function.arguments
                && !args.is_empty()
            {
                pending.arguments_json.push_str(&args);
                events.push(ResponseEvent::ToolDelta(args.into()));
            }
        }

        Ok(())
    }

    fn flush_tool_calls(&mut self, events: &mut Vec<ResponseEvent>) -> Result<(), Error> {
        for (sequence, tc) in std::mem::take(&mut self.tool_calls) {
            let call_id = tc.call_id.ok_or_else(|| {
                Error::Protocol(format!("missing call_id for tool call sequence {sequence}"))
            })?;
            let name = tc.name.ok_or_else(|| {
                Error::Protocol(format!("missing tool name for tool call '{call_id}'"))
            })?;

            match classify_tool_call(tc.call_type.as_deref(), Some(name.as_str())) {
                ToolCallKind::Mcp => {
                    let payload =
                        parse_zai_mcp_json(&tc.arguments_json, name.strip_prefix("__mcp__"))
                            .map_err(|err| {
                                Error::Protocol(format!(
                                    "invalid mcp payload for call '{call_id}' (sequence {sequence}): {err}"
                                ))
                            })?;

                    events.push(ResponseEvent::ToolDone(ToolCall::Mcp(McpCall {
                        sequence,
                        id: call_id,
                        mcp_type: payload.mcp_type,
                        server_label: payload.server_label,
                        name: payload.name,
                        arguments_json: payload.arguments_json,
                        output_json: payload.output_json,
                        tools_json: payload.tools_json,
                        error: payload.error,
                    })));
                }
                ToolCallKind::Builtin(builtin) => {
                    events.push(ResponseEvent::ToolDone(ToolCall::Builtin(BuiltinToolCall {
                        sequence,
                        call_id,
                        builtin,
                        arguments_json: tc.arguments_json,
                    })));
                }
                ToolCallKind::Function => {
                    events.push(ResponseEvent::ToolDone(ToolCall::FunctionCall {
                        sequence,
                        call_id,
                        name,
                        arguments_json: tc.arguments_json,
                    }));
                }
            }
            self.emitted_tool_sequences.insert(sequence);
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

        let mut events = Vec::with_capacity(TYPICAL_EVENTS_PER_CHUNK);
        self.flush_tool_calls(&mut events)?;
        self.emit_text_done_events(&mut events);
        events.push(ResponseEvent::Done(self.usage.take()));
        Ok(events)
    }
}

impl Default for Mapper {
    fn default() -> Self {
        Self::new()
    }
}
