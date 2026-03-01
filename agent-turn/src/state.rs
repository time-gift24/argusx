use std::collections::{HashMap, HashSet, VecDeque};

use agent_core::{InputEnvelope, SessionMeta, ToolCall, TranscriptItem, Usage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Active,
    Backoff,
    Done,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelState {
    NotStarted,
    Streaming,
    Completed,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 200,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnEngineConfig {
    pub max_parallel_tools: usize,
    pub retry_policy: RetryPolicy,
}

impl Default for TurnEngineConfig {
    fn default() -> Self {
        Self {
            max_parallel_tools: 4,
            retry_policy: RetryPolicy::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnState {
    pub meta: SessionMeta,
    pub lifecycle: Lifecycle,
    pub model_state: ModelState,
    pub epoch: u64,
    pub pending_inputs: VecDeque<InputEnvelope>,
    pub inflight_tools: HashMap<String, ToolCall>,
    pub output_buffer: String,
    pub reasoning_buffer: String,
    pub reasoning_started_emitted: bool,
    pub reasoning_completed_emitted: bool,
    pub reasoning_truncated: bool,
    pub reasoning_char_count: u32,
    pub usage: Usage,
    pub done_emitted: bool,
    pub retry_attempt: u32,
    pub seen_event_ids: HashSet<String>,
    pub transcript: Vec<TranscriptItem>,
    pub last_request_inputs: Vec<InputEnvelope>,
}

impl TurnState {
    pub fn new(meta: SessionMeta) -> Self {
        Self {
            meta,
            lifecycle: Lifecycle::Active,
            model_state: ModelState::NotStarted,
            epoch: 0,
            pending_inputs: VecDeque::new(),
            inflight_tools: HashMap::new(),
            output_buffer: String::new(),
            reasoning_buffer: String::new(),
            reasoning_started_emitted: false,
            reasoning_completed_emitted: false,
            reasoning_truncated: false,
            reasoning_char_count: 0,
            usage: Usage::default(),
            done_emitted: false,
            retry_attempt: 0,
            seen_event_ids: HashSet::new(),
            transcript: Vec::new(),
            last_request_inputs: Vec::new(),
        }
    }

    pub fn mark_seen(&mut self, event_id: &str) -> bool {
        !self.seen_event_ids.insert(event_id.to_string())
    }

    pub fn enqueue_input(&mut self, input: InputEnvelope) {
        self.pending_inputs.push_back(input);
    }

    pub fn drain_pending_inputs(&mut self) -> Vec<InputEnvelope> {
        self.pending_inputs.drain(..).collect()
    }

    pub fn can_finish(&self) -> bool {
        self.model_state == ModelState::Completed
            && self.inflight_tools.is_empty()
            && self.pending_inputs.is_empty()
            && !self.done_emitted
    }

    pub fn tool_calls_count(&self) -> u32 {
        self.transcript
            .iter()
            .filter(|item| matches!(item, TranscriptItem::ToolCall { .. }))
            .count() as u32
    }

    pub fn turn_id(&self) -> &str {
        &self.meta.turn_id
    }
}
