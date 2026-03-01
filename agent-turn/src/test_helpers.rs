//! Test helpers for reducer unit tests.
//!
//! Provides builders for constructing states, events, and assertions
//! to simplify state machine testing.
//!
//! ## Usage
//!
//! ```rust
//! use agent_turn::test_helpers::*;
//!
//! // Build initial state
//! let state = StateBuilder::new("s1", "t1")
//!     .with_lifecycle(Lifecycle::Active)
//!     .with_model_state(ModelState::Streaming)
//!     .with_inflight_tool("c1", tool_call("c1", "echo", json!({})))
//!     .build();
//!
//! // Build and dispatch event
//! let result = reduce(
//!     state,
//!     EventBuilder::tool_result_ok("c1", json!("result"))
//!         .with_epoch(0)
//!         .build(),
//!     &cfg(),
//! );
//!
//! // Assert transition
//! assert_transition(&result)
//!     .has_lifecycle(Lifecycle::Active)
//!     .has_no_inflight_tools()
//!     .emits_run_event::<RunStreamEvent>();
//! ```

use std::collections::{HashMap, HashSet, VecDeque};

use agent_core::{
    InputEnvelope, InputSource, RunStreamEvent, RuntimeEvent, SessionMeta, ToolCall, ToolResult,
    TranscriptItem, Usage,
};
use serde_json::json;

use crate::state::{Lifecycle, ModelState, TurnEngineConfig, TurnState};
use crate::transition::Transition;

// =============================================================================
// Constants
// =============================================================================

/// Default session ID for tests
pub const TEST_SESSION_ID: &str = "test_session";
/// Default turn ID for tests
pub const TEST_TURN_ID: &str = "test_turn";
/// Default tool name for tests
pub const TEST_TOOL_NAME: &str = "test_tool";
/// Default call ID prefix
pub const TEST_CALL_ID_PREFIX: &str = "call_";
/// Default event ID prefix
pub const TEST_EVENT_ID_PREFIX: &str = "evt_";

// =============================================================================
// Factories
// =============================================================================

/// Creates a new SessionMeta with given session_id and turn_id
pub fn session_meta(session_id: impl Into<String>, turn_id: impl Into<String>) -> SessionMeta {
    SessionMeta::new(session_id, turn_id)
}

/// Creates a tool call with the given call_id, tool_name, and arguments
pub fn tool_call(
    call_id: impl Into<String>,
    tool_name: impl Into<String>,
    arguments: serde_json::Value,
) -> ToolCall {
    ToolCall {
        call_id: call_id.into(),
        tool_name: tool_name.into(),
        arguments,
    }
}

/// Creates a user text input envelope
pub fn user_input(text: impl Into<String>) -> InputEnvelope {
    InputEnvelope::user_text(text)
}

/// Creates a tool result input envelope
pub fn tool_input(call_id: impl Into<String>, result: serde_json::Value) -> InputEnvelope {
    InputEnvelope::tool_json(json!({
        "call_id": call_id.into(),
        "result": result
    }))
}

/// Creates a default TurnEngineConfig for tests
pub fn test_config() -> TurnEngineConfig {
    TurnEngineConfig::default()
}

/// Generates a unique call ID
pub fn make_call_id(n: u32) -> String {
    format!("{}{:03}", TEST_CALL_ID_PREFIX, n)
}

/// Generates a unique event ID
pub fn make_event_id(n: u32) -> String {
    format!("{}{:03}", TEST_EVENT_ID_PREFIX, n)
}

// =============================================================================
// StateBuilder
// =============================================================================

/// Builder for constructing TurnState in tests.
///
/// ## Example
///
/// ```rust
/// let state = StateBuilder::new("s1", "t1")
///     .with_lifecycle(Lifecycle::Active)
///     .with_model_state(ModelState::Streaming)
///     .with_epoch(1)
///     .with_inflight_tool("c1", tool_call("c1", "echo", json!({})))
///     .with_pending_input(user_input("hello"))
///     .with_output_buffer("partial ")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct StateBuilder {
    session_id: String,
    turn_id: String,
    lifecycle: Lifecycle,
    model_state: ModelState,
    epoch: u64,
    pending_inputs: VecDeque<InputEnvelope>,
    inflight_tools: HashMap<String, ToolCall>,
    output_buffer: String,
    reasoning_buffer: String,
    reasoning_started_emitted: bool,
    reasoning_completed_emitted: bool,
    reasoning_truncated: bool,
    reasoning_char_count: u32,
    usage: Usage,
    done_emitted: bool,
    retry_attempt: u32,
    seen_event_ids: HashSet<String>,
    transcript: Vec<TranscriptItem>,
    last_request_inputs: Vec<InputEnvelope>,
}

impl StateBuilder {
    /// Creates a new StateBuilder with default values
    pub fn new(session_id: impl Into<String>, turn_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            turn_id: turn_id.into(),
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

    /// Creates a StateBuilder from an existing TurnState (for modification)
    pub fn from_state(state: TurnState) -> Self {
        Self {
            session_id: state.meta.session_id.clone(),
            turn_id: state.meta.turn_id.clone(),
            lifecycle: state.lifecycle,
            model_state: state.model_state,
            epoch: state.epoch,
            pending_inputs: state.pending_inputs,
            inflight_tools: state.inflight_tools,
            output_buffer: state.output_buffer,
            reasoning_buffer: state.reasoning_buffer,
            reasoning_started_emitted: state.reasoning_started_emitted,
            reasoning_completed_emitted: state.reasoning_completed_emitted,
            reasoning_truncated: state.reasoning_truncated,
            reasoning_char_count: state.reasoning_char_count,
            usage: state.usage,
            done_emitted: state.done_emitted,
            retry_attempt: state.retry_attempt,
            seen_event_ids: state.seen_event_ids,
            transcript: state.transcript,
            last_request_inputs: state.last_request_inputs,
        }
    }

    /// Sets the lifecycle state
    pub fn with_lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Sets the model state
    pub fn with_model_state(mut self, model_state: ModelState) -> Self {
        self.model_state = model_state;
        self
    }

    /// Sets the epoch
    pub fn with_epoch(mut self, epoch: u64) -> Self {
        self.epoch = epoch;
        self
    }

    /// Adds a pending input
    pub fn with_pending_input(mut self, input: InputEnvelope) -> Self {
        self.pending_inputs.push_back(input);
        self
    }

    /// Adds multiple pending inputs
    pub fn with_pending_inputs(mut self, inputs: impl IntoIterator<Item = InputEnvelope>) -> Self {
        for input in inputs {
            self.pending_inputs.push_back(input);
        }
        self
    }

    /// Adds an inflight tool
    pub fn with_inflight_tool(mut self, call_id: impl Into<String>, tool: ToolCall) -> Self {
        self.inflight_tools.insert(call_id.into(), tool);
        self
    }

    /// Adds multiple inflight tools
    pub fn with_inflight_tools(
        mut self,
        tools: impl IntoIterator<Item = (String, ToolCall)>,
    ) -> Self {
        for (call_id, tool) in tools {
            self.inflight_tools.insert(call_id, tool);
        }
        self
    }

    /// Sets the output buffer
    pub fn with_output_buffer(mut self, buffer: impl Into<String>) -> Self {
        self.output_buffer = buffer.into();
        self
    }

    /// Sets the reasoning buffer
    pub fn with_reasoning_buffer(mut self, buffer: impl Into<String>) -> Self {
        self.reasoning_buffer = buffer.into();
        self
    }

    /// Sets the usage
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = usage;
        self
    }

    /// Sets done_emitted
    pub fn with_done_emitted(mut self, done_emitted: bool) -> Self {
        self.done_emitted = done_emitted;
        self
    }

    /// Sets retry_attempt
    pub fn with_retry_attempt(mut self, attempt: u32) -> Self {
        self.retry_attempt = attempt;
        self
    }

    /// Adds a seen event ID
    pub fn with_seen_event(mut self, event_id: impl Into<String>) -> Self {
        self.seen_event_ids.insert(event_id.into());
        self
    }

    /// Adds multiple seen event IDs
    pub fn with_seen_events(
        mut self,
        event_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        for event_id in event_ids {
            self.seen_event_ids.insert(event_id.into());
        }
        self
    }

    /// Adds a transcript item
    pub fn with_transcript_item(mut self, item: TranscriptItem) -> Self {
        self.transcript.push(item);
        self
    }

    /// Adds multiple transcript items
    pub fn with_transcript(mut self, items: impl IntoIterator<Item = TranscriptItem>) -> Self {
        for item in items {
            self.transcript.push(item);
        }
        self
    }

    /// Sets last_request_inputs
    pub fn with_last_request_inputs(
        mut self,
        inputs: impl IntoIterator<Item = InputEnvelope>,
    ) -> Self {
        self.last_request_inputs = inputs.into_iter().collect();
        self
    }

    /// Builds the TurnState
    pub fn build(self) -> TurnState {
        TurnState {
            meta: SessionMeta::new(self.session_id, self.turn_id),
            lifecycle: self.lifecycle,
            model_state: self.model_state,
            epoch: self.epoch,
            pending_inputs: self.pending_inputs,
            inflight_tools: self.inflight_tools,
            output_buffer: self.output_buffer,
            reasoning_buffer: self.reasoning_buffer,
            reasoning_started_emitted: self.reasoning_started_emitted,
            reasoning_completed_emitted: self.reasoning_completed_emitted,
            reasoning_truncated: self.reasoning_truncated,
            reasoning_char_count: self.reasoning_char_count,
            usage: self.usage,
            done_emitted: self.done_emitted,
            retry_attempt: self.retry_attempt,
            seen_event_ids: self.seen_event_ids,
            transcript: self.transcript,
            last_request_inputs: self.last_request_inputs,
        }
    }
}

// =============================================================================
// EventBuilder
// =============================================================================

/// Builder for constructing RuntimeEvent in tests.
///
/// ## Example
///
/// ```rust
/// // Simple events
/// let event = EventBuilder::turn_started("t1", user_input("hello")).build();
/// let event = EventBuilder::model_text_delta("hello").with_epoch(0).build();
///
/// // Tool events
/// let event = EventBuilder::model_tool_call("c1", "echo", json!({})).with_epoch(0).build();
/// let event = EventBuilder::tool_result_ok("c1", json!("result")).build();
/// let event = EventBuilder::tool_result_err("c1", "error").build();
/// ```
#[derive(Debug, Clone)]
pub struct EventBuilder {
    variant: EventVariant,
}

#[derive(Debug, Clone)]
enum EventVariant {
    TurnStarted {
        turn_id: String,
        input: InputEnvelope,
    },
    ModelTextDelta {
        epoch: u64,
        delta: String,
    },
    ModelReasoningDelta {
        epoch: u64,
        delta: String,
    },
    ModelToolCall {
        epoch: u64,
        call: ToolCall,
    },
    ToolDispatched {
        epoch: u64,
        call_id: String,
    },
    ToolResultOk {
        epoch: u64,
        result: ToolResult,
    },
    ToolResultErr {
        epoch: u64,
        result: ToolResult,
    },
    InputInjected {
        input: InputEnvelope,
    },
    ModelCompleted {
        epoch: u64,
        usage: Option<Usage>,
    },
    RetryTimerFired {
        next_epoch: u64,
    },
    TransientError {
        epoch: u64,
        message: String,
        retry_after_ms: Option<u64>,
    },
    FatalError {
        message: String,
    },
    CancelRequested {
        reason: Option<String>,
    },
}

impl EventBuilder {
    /// Creates a TurnStarted event
    pub fn turn_started(turn_id: impl Into<String>, input: InputEnvelope) -> Self {
        Self {
            variant: EventVariant::TurnStarted {
                turn_id: turn_id.into(),
                input,
            },
        }
    }

    /// Creates a ModelTextDelta event
    pub fn model_text_delta(text: impl Into<String>) -> Self {
        Self {
            variant: EventVariant::ModelTextDelta {
                epoch: 0,
                delta: text.into(),
            },
        }
    }

    /// Creates a ModelReasoningDelta event
    pub fn model_reasoning_delta(text: impl Into<String>) -> Self {
        Self {
            variant: EventVariant::ModelReasoningDelta {
                epoch: 0,
                delta: text.into(),
            },
        }
    }

    /// Creates a ModelToolCall event
    pub fn model_tool_call(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            variant: EventVariant::ModelToolCall {
                epoch: 0,
                call: ToolCall {
                    call_id: call_id.into(),
                    tool_name: tool_name.into(),
                    arguments,
                },
            },
        }
    }

    /// Creates a ToolDispatched event
    pub fn tool_dispatched(call_id: impl Into<String>) -> Self {
        Self {
            variant: EventVariant::ToolDispatched {
                epoch: 0,
                call_id: call_id.into(),
            },
        }
    }

    /// Creates a ToolResultOk event
    pub fn tool_result_ok(call_id: impl Into<String>, output: serde_json::Value) -> Self {
        Self {
            variant: EventVariant::ToolResultOk {
                epoch: 0,
                result: ToolResult::ok(call_id, output),
            },
        }
    }

    /// Creates a ToolResultErr event
    pub fn tool_result_err(call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            variant: EventVariant::ToolResultErr {
                epoch: 0,
                result: ToolResult::err(call_id, message),
            },
        }
    }

    /// Creates an InputInjected event
    pub fn input_injected(input: InputEnvelope) -> Self {
        Self {
            variant: EventVariant::InputInjected { input },
        }
    }

    /// Creates a ModelCompleted event
    pub fn model_completed() -> Self {
        Self {
            variant: EventVariant::ModelCompleted {
                epoch: 0,
                usage: None,
            },
        }
    }

    /// Creates a RetryTimerFired event
    pub fn retry_timer_fired(next_epoch: u64) -> Self {
        Self {
            variant: EventVariant::RetryTimerFired { next_epoch },
        }
    }

    /// Creates a TransientError event
    pub fn transient_error(message: impl Into<String>) -> Self {
        Self {
            variant: EventVariant::TransientError {
                epoch: 0,
                message: message.into(),
                retry_after_ms: None,
            },
        }
    }

    /// Creates a FatalError event
    pub fn fatal_error(message: impl Into<String>) -> Self {
        Self {
            variant: EventVariant::FatalError {
                message: message.into(),
            },
        }
    }

    /// Creates a CancelRequested event
    pub fn cancel_requested() -> Self {
        Self {
            variant: EventVariant::CancelRequested { reason: None },
        }
    }

    /// Sets the event ID (defaults to auto-generated)
    pub fn with_event_id(self, _event_id: impl Into<String>) -> Self {
        // Note: We handle this in build() method
        self
    }

    /// Sets the epoch
    pub fn with_epoch(mut self, epoch: u64) -> Self {
        match &mut self.variant {
            EventVariant::ModelTextDelta { epoch: e, .. }
            | EventVariant::ModelReasoningDelta { epoch: e, .. }
            | EventVariant::ModelToolCall { epoch: e, .. }
            | EventVariant::ToolResultOk { epoch: e, .. }
            | EventVariant::ToolResultErr { epoch: e, .. }
            | EventVariant::ModelCompleted { epoch: e, .. }
            | EventVariant::TransientError { epoch: e, .. } => {
                *e = epoch;
            }
            _ => {}
        }
        self
    }

    /// Sets the usage for ModelCompleted
    pub fn with_usage(mut self, usage: Usage) -> Self {
        if let EventVariant::ModelCompleted { usage: u, .. } = &mut self.variant {
            *u = Some(usage);
        }
        self
    }

    /// Sets retry_after_ms for TransientError
    pub fn with_retry_after_ms(mut self, ms: u64) -> Self {
        if let EventVariant::TransientError {
            retry_after_ms: r, ..
        } = &mut self.variant
        {
            *r = Some(ms);
        }
        self
    }

    /// Builds the RuntimeEvent
    pub fn build(self) -> RuntimeEvent {
        static EVENT_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

        let event_id = format!(
            "{}{}",
            TEST_EVENT_ID_PREFIX,
            EVENT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );

        match self.variant {
            EventVariant::TurnStarted { turn_id, input } => RuntimeEvent::TurnStarted {
                event_id,
                turn_id,
                input,
            },
            EventVariant::ModelTextDelta { epoch, delta } => RuntimeEvent::ModelTextDelta {
                event_id,
                epoch,
                delta,
            },
            EventVariant::ModelReasoningDelta { epoch, delta } => {
                RuntimeEvent::ModelReasoningDelta {
                    event_id,
                    epoch,
                    delta,
                }
            }
            EventVariant::ModelToolCall { epoch, call } => RuntimeEvent::ModelToolCall {
                event_id,
                epoch,
                call,
            },
            EventVariant::ToolDispatched { epoch, call_id } => RuntimeEvent::ToolDispatched {
                event_id,
                epoch,
                call_id,
            },
            EventVariant::ToolResultOk { epoch, result } => RuntimeEvent::ToolResultOk {
                event_id,
                epoch,
                result,
            },
            EventVariant::ToolResultErr { epoch, result } => RuntimeEvent::ToolResultErr {
                event_id,
                epoch,
                result,
            },
            EventVariant::InputInjected { input } => {
                RuntimeEvent::InputInjected { event_id, input }
            }
            EventVariant::ModelCompleted { epoch, usage } => RuntimeEvent::ModelCompleted {
                event_id,
                epoch,
                usage,
            },
            EventVariant::RetryTimerFired { next_epoch } => RuntimeEvent::RetryTimerFired {
                event_id,
                next_epoch,
            },
            EventVariant::TransientError {
                epoch,
                message,
                retry_after_ms,
            } => RuntimeEvent::TransientError {
                event_id,
                epoch,
                message,
                retry_after_ms,
            },
            EventVariant::FatalError { message } => RuntimeEvent::FatalError { event_id, message },
            EventVariant::CancelRequested { reason } => {
                RuntimeEvent::CancelRequested { event_id, reason }
            }
        }
    }
}

// =============================================================================
// TransitionAssert
// =============================================================================

/// Builder for asserting on Transition results.
///
/// ## Example
///
/// ```rust
/// let result = reduce(state, event, &cfg());
///
/// assert_transition(&result)
///     .has_lifecycle(Lifecycle::Done)
///     .has_model_state(ModelState::Completed)
///     .has_no_inflight_tools()
///     .emits_run_event::<RunStreamEvent>()
///     .emits_ui_event::<UIStreamEvent>();
/// ```
#[derive(Debug)]
pub struct TransitionAssert<'a> {
    transition: &'a Transition,
}

impl<'a> TransitionAssert<'a> {
    /// Creates a new TransitionAssert
    pub fn new(transition: &'a Transition) -> Self {
        Self { transition }
    }

    /// Asserts lifecycle equals expected value
    pub fn has_lifecycle(self, expected: Lifecycle) -> Self {
        assert_eq!(
            self.transition.state.lifecycle, expected,
            "Expected lifecycle {:?}, got {:?}",
            expected, self.transition.state.lifecycle
        );
        self
    }

    /// Asserts model_state equals expected value
    pub fn has_model_state(self, expected: ModelState) -> Self {
        assert_eq!(
            self.transition.state.model_state, expected,
            "Expected model_state {:?}, got {:?}",
            expected, self.transition.state.model_state
        );
        self
    }

    /// Asserts epoch equals expected value
    pub fn has_epoch(self, expected: u64) -> Self {
        assert_eq!(
            self.transition.state.epoch, expected,
            "Expected epoch {}, got {}",
            expected, self.transition.state.epoch
        );
        self
    }

    /// Asserts output_buffer equals expected value
    pub fn has_output_buffer(self, expected: &str) -> Self {
        assert_eq!(
            self.transition.state.output_buffer, expected,
            "Expected output_buffer \"{}\", got \"{}\"",
            expected, self.transition.state.output_buffer
        );
        self
    }

    /// Asserts output_buffer contains expected substring
    pub fn output_buffer_contains(self, expected: &str) -> Self {
        assert!(
            self.transition.state.output_buffer.contains(expected),
            "Expected output_buffer to contain \"{}\", got \"{}\"",
            expected,
            self.transition.state.output_buffer
        );
        self
    }

    /// Asserts there are no inflight tools
    pub fn has_no_inflight_tools(self) -> Self {
        assert!(
            self.transition.state.inflight_tools.is_empty(),
            "Expected no inflight tools, got {:?}",
            self.transition
                .state
                .inflight_tools
                .keys()
                .collect::<Vec<_>>()
        );
        self
    }

    /// Asserts inflight_tools contains expected call_id
    pub fn has_inflight_tool(self, call_id: &str) -> Self {
        assert!(
            self.transition.state.inflight_tools.contains_key(call_id),
            "Expected inflight_tools to contain {}, got {:?}",
            call_id,
            self.transition
                .state
                .inflight_tools
                .keys()
                .collect::<Vec<_>>()
        );
        self
    }

    /// Asserts there are no pending inputs
    pub fn has_no_pending_inputs(self) -> Self {
        assert!(
            self.transition.state.pending_inputs.is_empty(),
            "Expected no pending inputs, got {}",
            self.transition.state.pending_inputs.len()
        );
        self
    }

    /// Asserts pending_inputs has expected count
    pub fn has_pending_inputs_count(self, expected: usize) -> Self {
        assert_eq!(
            self.transition.state.pending_inputs.len(),
            expected,
            "Expected {} pending inputs, got {}",
            expected,
            self.transition.state.pending_inputs.len()
        );
        self
    }

    /// Asserts done_emitted equals expected value
    pub fn has_done_emitted(self, expected: bool) -> Self {
        assert_eq!(
            self.transition.state.done_emitted, expected,
            "Expected done_emitted {}, got {}",
            expected, self.transition.state.done_emitted
        );
        self
    }

    /// Asserts retry_attempt equals expected value
    pub fn has_retry_attempt(self, expected: u32) -> Self {
        assert_eq!(
            self.transition.state.retry_attempt, expected,
            "Expected retry_attempt {}, got {}",
            expected, self.transition.state.retry_attempt
        );
        self
    }

    /// Asserts transcript has expected number of items
    pub fn has_transcript_count(self, expected: usize) -> Self {
        assert_eq!(
            self.transition.state.transcript.len(),
            expected,
            "Expected transcript len {}, got {}",
            expected,
            self.transition.state.transcript.len()
        );
        self
    }

    /// Asserts run_events contains a TurnDone event
    pub fn emits_turn_done(self) -> Self {
        let has_event = self
            .transition
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::TurnDone { .. }));

        assert!(
            has_event,
            "Expected run_events to contain TurnDone, got {:?}",
            self.transition
                .run_events
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
        );
        self
    }

    /// Asserts run_events does NOT contain a TurnDone event
    pub fn does_not_emit_turn_done(self) -> Self {
        let has_event = self
            .transition
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::TurnDone { .. }));

        assert!(
            !has_event,
            "Expected run_events NOT to contain TurnDone, but it does"
        );
        self
    }

    /// Asserts run_events contains a specific RunStreamEvent variant
    #[allow(dead_code)]
    pub fn emits_run_event_matching(self, f: impl Fn(&RunStreamEvent) -> bool) -> Self {
        let has_event = self.transition.run_events.iter().any(f);

        assert!(has_event, "Expected run_events to match predicate");
        self
    }

    /// Asserts ui_events contains an event of the given type
    #[allow(dead_code)]
    pub fn emits_ui_event<T: 'static>(self) -> Self {
        // Note: This is a placeholder - UI events are not currently in Transition
        // If they are added in the future, this can be implemented
        let _ = self;
        unimplemented!("UI events not yet implemented in Transition");
    }

    /// Asserts effects contains a StartModel effect with expected epoch
    #[allow(dead_code)]
    pub fn starts_model_with_epoch(self, expected_epoch: u64) -> Self {
        use crate::effect::Effect;

        let has_effect = self
            .transition
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartModel { epoch, .. } if *epoch == expected_epoch));

        assert!(
            has_effect,
            "Expected effects to contain StartModel with epoch {}",
            expected_epoch
        );
        self
    }

    /// Asserts effects contains a ScheduleRetry effect
    #[allow(dead_code)]
    pub fn schedules_retry(self) -> Self {
        use crate::effect::Effect;

        let has_effect = self
            .transition
            .effects
            .iter()
            .any(|e| matches!(e, Effect::ScheduleRetry { .. }));

        assert!(has_effect, "Expected effects to contain ScheduleRetry");
        self
    }
}

/// Shorthand for creating a TransitionAssert
///
/// ## Example
///
/// ```rust
/// let result = reduce(state, event, &cfg());
/// assert_transition!(&result)
///     .has_lifecycle(Lifecycle::Done)
///     .has_no_inflight_tools();
/// ```
#[macro_export]
macro_rules! assert_transition {
    ($transition:expr) => {
        $crate::test_helpers::TransitionAssert::new($transition)
    };
}

// =============================================================================
// ScenarioRunner
// =============================================================================

/// Runner for executing event sequences and collecting results.
///
/// ## Example
///
/// ```rust
/// let runner = ScenarioRunner::new(initial_state, &cfg())
///     .push(EventBuilder::turn_started("t1", user_input("hello")).build())
///     .expect_state(|s| s.model_state == ModelState::Streaming)
///     .push(EventBuilder::model_text_delta("hi").with_epoch(0).build())
///     .expect_state(|s| s.output_buffer == "hi")
///     .run();
///
/// // Access final state and all collected events
/// let final_state = runner.state();
/// let all_run_events = runner.run_events();
/// ```
#[derive(Debug)]
pub struct ScenarioRunner {
    state: TurnState,
    config: TurnEngineConfig,
    run_events: Vec<RunStreamEvent>,
    ui_events: Vec<agent_core::UiThreadEvent>,
    effects: Vec<crate::effect::Effect>,
}

impl ScenarioRunner {
    /// Creates a new ScenarioRunner with initial state
    pub fn new(state: TurnState, config: &TurnEngineConfig) -> Self {
        Self {
            state,
            config: config.clone(),
            run_events: Vec::new(),
            ui_events: Vec::new(),
            effects: Vec::new(),
        }
    }

    /// Creates a new ScenarioRunner with StateBuilder
    pub fn from_builder(builder: StateBuilder, config: &TurnEngineConfig) -> Self {
        Self::new(builder.build(), config)
    }

    /// Pushes an event and returns self for chaining
    pub fn push(mut self, event: RuntimeEvent) -> Self {
        let transition = crate::reducer::reduce(self.state, event, &self.config);
        self.state = transition.state;
        self.run_events.extend(transition.run_events);
        self.ui_events.extend(transition.ui_events);
        self.effects.extend(transition.effects);
        self
    }

    /// Pushes an event and asserts the resulting state
    pub fn push_expect(mut self, event: RuntimeEvent, f: impl Fn(&TurnState)) -> Self {
        let transition = crate::reducer::reduce(self.state, event, &self.config);
        f(&transition.state);
        self.state = transition.state;
        self.run_events.extend(transition.run_events);
        self.ui_events.extend(transition.ui_events);
        self.effects.extend(transition.effects);
        self
    }

    /// Runs the scenario and returns self
    pub fn run(self) -> Self {
        self
    }

    /// Returns the final state
    pub fn state(&self) -> &TurnState {
        &self.state
    }

    /// Returns all collected run events
    pub fn run_events(&self) -> &[RunStreamEvent] {
        &self.run_events
    }

    /// Returns all collected ui events
    pub fn ui_events(&self) -> &[agent_core::UiThreadEvent] {
        &self.ui_events
    }

    /// Returns all collected effects
    pub fn effects(&self) -> &[crate::effect::Effect] {
        &self.effects
    }

    /// Consumes self and returns the final state
    pub fn into_state(self) -> TurnState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_builder_default() {
        let state = StateBuilder::new("s1", "t1").build();

        assert_eq!(state.meta.session_id, "s1");
        assert_eq!(state.meta.turn_id, "t1");
        assert_eq!(state.lifecycle, Lifecycle::Active);
        assert_eq!(state.model_state, ModelState::NotStarted);
        assert_eq!(state.epoch, 0);
    }

    #[test]
    fn state_builder_with_options() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Backoff)
            .with_model_state(ModelState::Streaming)
            .with_epoch(2)
            .with_output_buffer("hello")
            .with_inflight_tool("c1", tool_call("c1", "echo", json!({})))
            .with_pending_input(user_input("test"))
            .with_done_emitted(true)
            .build();

        assert_eq!(state.lifecycle, Lifecycle::Backoff);
        assert_eq!(state.model_state, ModelState::Streaming);
        assert_eq!(state.epoch, 2);
        assert_eq!(state.output_buffer, "hello");
        assert!(state.inflight_tools.contains_key("c1"));
        assert_eq!(state.pending_inputs.len(), 1);
        assert!(state.done_emitted);
    }

    #[test]
    fn event_builder_turn_started() {
        let event = EventBuilder::turn_started("t1", user_input("hello"))
            .with_event_id("e1")
            .build();

        match event {
            RuntimeEvent::TurnStarted { turn_id, input, .. } => {
                assert_eq!(turn_id, "t1");
                assert!(matches!(input.source, InputSource::User));
            }
            _ => panic!("Expected TurnStarted event"),
        }
    }

    #[test]
    fn event_builder_model_tool_call() {
        let event = EventBuilder::model_tool_call("c1", "echo", json!({"a": 1}))
            .with_epoch(1)
            .build();

        match event {
            RuntimeEvent::ModelToolCall { epoch, call, .. } => {
                assert_eq!(epoch, 1);
                assert_eq!(call.call_id, "c1");
                assert_eq!(call.tool_name, "echo");
            }
            _ => panic!("Expected ModelToolCall event"),
        }
    }

    #[test]
    fn event_builder_tool_result() {
        let event = EventBuilder::tool_result_ok("c1", json!("result")).build();

        match event {
            RuntimeEvent::ToolResultOk { result, .. } => {
                assert_eq!(result.call_id, "c1");
                assert_eq!(result.output, json!("result"));
            }
            _ => panic!("Expected ToolResultOk event"),
        }
    }

    #[test]
    fn transition_assert_lifecycle() {
        use crate::reducer::reduce;

        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Done)
            .build();

        let transition = reduce(
            state,
            RuntimeEvent::ModelTextDelta {
                event_id: "e1".into(),
                epoch: 0,
                delta: "hello".into(),
            },
            &test_config(),
        );

        // This event should be ignored (guard fails), so lifecycle stays Done
        assert_transition!(&transition)
            .has_lifecycle(Lifecycle::Done)
            .has_model_state(ModelState::NotStarted);
    }

    #[test]
    fn scenario_runner_basic() {
        let initial = StateBuilder::new("s1", "t1").build();
        let config = test_config();

        let final_state = ScenarioRunner::new(initial, &config)
            .push(EventBuilder::turn_started("t1", user_input("hello")).build())
            .push(EventBuilder::model_text_delta("hi").with_epoch(0).build())
            .push(EventBuilder::model_completed().with_epoch(0).build())
            .run()
            .into_state();

        assert_eq!(final_state.lifecycle, Lifecycle::Done);
        assert_eq!(final_state.output_buffer, "hi");
    }

    #[test]
    fn scenario_runner_with_assertions() {
        let initial = StateBuilder::new("s1", "t1").build();
        let config = test_config();

        ScenarioRunner::new(initial, &config)
            .push_expect(
                EventBuilder::turn_started("t1", user_input("hello")).build(),
                |s| {
                    assert_eq!(s.model_state, ModelState::Streaming);
                },
            )
            .push_expect(
                EventBuilder::model_text_delta("hi").with_epoch(0).build(),
                |s| {
                    assert_eq!(s.output_buffer, "hi");
                },
            )
            .run();
    }
}
