use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::sync::broadcast;
use turn::{PermissionDecision, TurnDriver, TurnEvent, TurnHandle, TurnOutcome, TurnSeed};
use uuid::Uuid;

use crate::database::DynSessionDatabase;
use crate::error::{SessionError, SessionResult};
use crate::manager::TurnDependencies;
use crate::thread::{persist_tool_call, persist_transcript, ActiveTurnRuntime, ThreadRuntime};
use crate::types::{PersistedMessage, ThreadEvent, ThreadEventEnvelope, ThreadLifecycle, ThreadRecord, TurnRecord, TurnStatus};

/// Thread aggregate - encapsulates thread state and event channel.
/// This is the primary interface for interacting with a specific thread.
#[allow(dead_code)]
pub struct Thread {
    record: ThreadRecord,
    session_id: String,
    event_tx: broadcast::Sender<ThreadEventEnvelope>,
    db: DynSessionDatabase,
    deps: Option<Arc<TurnDependencies>>,
    runtime: Arc<Mutex<ThreadRuntime>>,
    /// Optional channel to forward events to SessionManager.
    /// This enables the desktop event bridge to receive turn events.
    session_events_tx: Option<broadcast::Sender<crate::manager::SessionEvent>>,
}

impl Thread {
    /// Create a new Thread with the given dependencies.
    pub fn new(
        record: ThreadRecord,
        session_id: String,
        event_tx: broadcast::Sender<ThreadEventEnvelope>,
        db: DynSessionDatabase,
        deps: Option<Arc<TurnDependencies>>,
        session_events_tx: Option<broadcast::Sender<crate::manager::SessionEvent>>,
    ) -> Self {
        let thread_id = record.id;
        Self {
            record,
            session_id,
            event_tx,
            db,
            deps,
            runtime: Arc::new(Mutex::new(ThreadRuntime::new(thread_id))),
            session_events_tx,
        }
    }

    /// Returns the thread record.
    pub fn record(&self) -> &ThreadRecord {
        &self.record
    }

    /// Returns the thread ID.
    pub fn id(&self) -> Uuid {
        self.record.id
    }

    /// Subscribe to thread-scoped events.
    pub fn subscribe(&self) -> broadcast::Receiver<ThreadEventEnvelope> {
        self.event_tx.subscribe()
    }

    /// Emit an event to all subscribers of this thread.
    pub fn emit(&self, event: ThreadEvent, turn_id: Option<Uuid>) {
        let _ = self.event_tx.send(ThreadEventEnvelope {
            thread_id: self.id(),
            turn_id,
            event,
        });
    }

    /// Check if this thread has an active turn.
    pub fn has_active_turn(&self) -> bool {
        self.runtime.lock().unwrap().active_turn.is_some()
    }

    /// Get a clone of the runtime for spawning turn bridges.
    pub fn runtime(&self) -> Arc<Mutex<ThreadRuntime>> {
        Arc::clone(&self.runtime)
    }

    /// Send a message to start a new turn.
    pub async fn send_message(&self, content: String) -> SessionResult<Uuid> {
        self.send_message_with_turn_id(Uuid::new_v4(), content).await
    }

    /// Send a message with a specific turn ID.
    pub async fn send_message_with_turn_id(&self, turn_id: Uuid, content: String) -> SessionResult<Uuid> {
        let deps = self.deps.as_ref()
            .ok_or_else(|| SessionError::TurnMessage("no dependencies configured".to_string()))?;

        // Reserve the turn slot to prevent concurrent turns
        self.reserve_turn(turn_id)?;

        // Load history and get turn number
        let history = self.db.list_turns(self.record.id).await?;
        let thread = self.db.get_thread(self.record.id).await?
            .ok_or_else(|| SessionError::ThreadNotFound(self.record.id))?;

        let (turn_number, prior_messages, prior_message_count) = {
            let runtime = self.runtime.lock().unwrap();
            let turn_number = thread.last_turn_number + 1;
            let prior_messages = runtime.build_prior_messages(&history);
            let prior_message_count = prior_messages.len();
            (turn_number, prior_messages, prior_message_count)
        };

        // Create turn record
        let now = Utc::now();
        let turn_record = TurnRecord {
            id: turn_id,
            thread_id: self.record.id,
            turn_number,
            user_input: content.clone(),
            status: TurnStatus::Running,
            finish_reason: None,
            transcript: vec![PersistedMessage::User { content: content.clone() }],
            final_output: None,
            started_at: now,
            finished_at: None,
        };

        // Insert turn and advance thread
        self.db.insert_turn_and_advance_thread(&turn_record, thread.last_turn_number, now).await?;

        // Spawn the turn driver
        let seed = TurnSeed {
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            prior_messages,
            user_message: content,
        };
        let (handle, task) = TurnDriver::spawn(seed, deps.model.clone(), deps.tool_runner.clone(), deps.authorizer.clone());

        // Activate the reservation
        self.activate_turn(turn_id, turn_number, handle.controller())?;

        // Spawn the bridge to handle events
        self.spawn_turn_bridge(turn_record, prior_message_count, handle, task);

        Ok(turn_id)
    }

    /// Resolve a pending permission request.
    pub async fn resolve_permission(&self, request_id: String, decision: PermissionDecision) -> SessionResult<()> {
        let controller = self.active_turn_controller()?;
        controller.resolve_permission(request_id, decision).await
            .map_err(|e| SessionError::TurnMessage(e.to_string()))?;
        Ok(())
    }

    /// Cancel the active turn.
    pub async fn cancel_turn(&self) -> SessionResult<()> {
        let controller = self.active_turn_controller()?;
        controller.cancel().await
            .map_err(|e| SessionError::TurnMessage(e.to_string()))?;
        Ok(())
    }

    /// Load turn history for this thread.
    pub async fn load_history(&self) -> SessionResult<Vec<TurnRecord>> {
        self.db.list_turns(self.record.id).await
    }

    // Private methods

    fn reserve_turn(&self, turn_id: Uuid) -> SessionResult<()> {
        let mut runtime = self.runtime.lock().unwrap();

        if runtime.active_turn.is_some() {
            return Err(SessionError::TurnAlreadyActive);
        }

        runtime.active_turn = Some(ActiveTurnRuntime::starting(turn_id));
        Ok(())
    }

    fn activate_turn(&self, turn_id: Uuid, turn_number: u32, controller: turn::TurnController) -> SessionResult<()> {
        let mut runtime = self.runtime.lock().unwrap();

        let active_turn = runtime.active_turn.as_mut()
            .ok_or_else(|| SessionError::NoActiveTurn)?;

        if active_turn.turn_id != turn_id {
            return Err(SessionError::TurnMessage("turn ID mismatch".to_string()));
        }

        active_turn.activate(turn_number, controller);
        Ok(())
    }

    fn active_turn_controller(&self) -> SessionResult<turn::TurnController> {
        let runtime = self.runtime.lock().unwrap();

        let active_turn = runtime.active_turn.as_ref()
            .ok_or_else(|| SessionError::NoActiveTurn)?;

        active_turn.controller.clone()
            .ok_or_else(|| SessionError::TurnMessage("turn controller not ready".to_string()))
    }

    fn spawn_turn_bridge(
        &self,
        mut turn_record: TurnRecord,
        prior_message_count: usize,
        handle: TurnHandle,
        task: tokio::task::JoinHandle<Result<TurnOutcome, turn::TurnError>>,
    ) {
        let db = Arc::clone(&self.db);
        let event_tx = self.event_tx.clone();
        let runtime = Arc::clone(&self.runtime);
        let thread_id = self.record.id;
        let session_events_tx = self.session_events_tx.clone();

        tokio::spawn(async move {
            let mut transcript = IncrementalTranscript::new(turn_record.transcript.clone());

            // Process turn events
            while let Some(event) = handle.next_event().await {
                transcript.apply_event(&event);

                match &event {
                    TurnEvent::ToolCallPermissionRequested { request } => {
                        turn_record.status = TurnStatus::WaitingPermission;
                        turn_record.transcript = transcript.snapshot();
                        {
                            let mut rt = runtime.lock().unwrap();
                            if let Some(at) = rt.active_turn.as_mut()
                                && at.turn_id == turn_record.id
                            {
                                at.waiting_permission = Some(request.clone());
                            }
                        }
                        let _ = db.update_turn(&turn_record).await;
                    }
                    TurnEvent::ToolCallPermissionResolved { .. } => {
                        turn_record.status = TurnStatus::Running;
                        turn_record.transcript = transcript.snapshot();
                        {
                            let mut rt = runtime.lock().unwrap();
                            if let Some(at) = rt.active_turn.as_mut()
                                && at.turn_id == turn_record.id
                            {
                                at.waiting_permission = None;
                            }
                        }
                        let _ = db.update_turn(&turn_record).await;
                    }
                    _ => {}
                }

                // Forward turn event to thread-scoped channel
                let _ = event_tx.send(ThreadEventEnvelope {
                    thread_id,
                    turn_id: Some(turn_record.id),
                    event: ThreadEvent::TurnEventForwarded,
                });

                // Forward turn event to session-scoped channel (for desktop event bridge)
                if let Some(ref tx) = session_events_tx {
                    let _ = tx.send(crate::manager::SessionEvent::Turn {
                        thread_id,
                        turn_id: turn_record.id,
                        event: event.clone(),
                    });
                }
            }

            // Handle turn completion
            match task.await {
                Ok(Ok(outcome)) => {
                    apply_turn_outcome(&mut turn_record, prior_message_count, outcome);
                    let _ = db.update_turn(&turn_record).await;
                }
                Ok(Err(_)) => {
                    turn_record.status = TurnStatus::Failed;
                    turn_record.finish_reason = Some("Failed".into());
                    turn_record.transcript = transcript.snapshot();
                    turn_record.finished_at = Some(Utc::now());
                    let _ = db.update_turn(&turn_record).await;
                }
                Err(_) => {
                    turn_record.status = TurnStatus::Failed;
                    turn_record.finish_reason = Some("Failed".into());
                    turn_record.transcript = transcript.snapshot();
                    turn_record.finished_at = Some(Utc::now());
                    let _ = db.update_turn(&turn_record).await;
                }
            }

            // Clear active turn
            {
                let mut rt = runtime.lock().unwrap();
                if rt.active_turn.as_ref().map(|at| at.turn_id == turn_record.id).unwrap_or(false)
                {
                    rt.active_turn = None;
                }
            }

            let _ = event_tx.send(ThreadEventEnvelope {
                thread_id,
                turn_id: Some(turn_record.id),
                event: ThreadEvent::ThreadUpdated,
            });

            // Forward thread updated event to session
            if let Some(ref tx) = session_events_tx {
                let _ = tx.send(crate::manager::SessionEvent::Thread {
                    thread_id,
                    event: ThreadEvent::ThreadUpdated,
                });
            }
        });
    }
}

pub struct Session {
    session_id: String,
    active_thread_id: Option<Uuid>,
    /// Thread aggregates owned by this session.
    /// Each Arc<Thread> is shared between Session internals and external callers.
    threads: HashMap<Uuid, Arc<Thread>>,
    db: DynSessionDatabase,
    /// Dependencies for turn execution.
    deps: Option<Arc<TurnDependencies>>,
}

impl Session {
    pub fn new(session_id: String, db: DynSessionDatabase) -> Self {
        Self {
            session_id,
            active_thread_id: None,
            threads: HashMap::new(),
            db,
            deps: None,
        }
    }

    pub fn set_dependencies(&mut self, deps: Arc<TurnDependencies>) {
        self.deps = Some(deps);
    }

    pub async fn create_thread(&mut self, title: Option<String>) -> SessionResult<Uuid> {
        let thread_id = Uuid::new_v4();
        let now = Utc::now();

        let record = ThreadRecord {
            id: thread_id,
            session_id: self.session_id.clone(),
            title,
            lifecycle: ThreadLifecycle::Open,
            created_at: now,
            updated_at: now,
            last_turn_number: 0,
        };

        self.db.insert_thread(&record).await?;

        let (event_tx, _) = broadcast::channel(64);
        let deps = self.deps.clone();
        let thread = Thread::new(
            record,
            self.session_id.clone(),
            event_tx,
            Arc::clone(&self.db),
            deps,
            None, // Session doesn't forward events to a parent
        );

        self.threads.insert(thread_id, Arc::new(thread));
        self.active_thread_id = Some(thread_id);

        Ok(thread_id)
    }

    pub async fn switch_thread(&mut self, thread_id: Uuid) -> SessionResult<()> {
        if self.get_thread(thread_id)?.is_none() && self.load_thread(thread_id).await?.is_none() {
            return Err(SessionError::ThreadNotFound(thread_id));
        }

        self.active_thread_id = Some(thread_id);

        let thread = self.threads.get(&thread_id).unwrap();
        thread.emit(ThreadEvent::ThreadActivated, None);

        Ok(())
    }

    pub async fn list_threads(&self) -> SessionResult<Vec<ThreadRecord>> {
        self.db.list_threads(&self.session_id).await
    }

    pub async fn subscribe(
        &mut self,
        thread_id: Uuid,
    ) -> SessionResult<broadcast::Receiver<ThreadEventEnvelope>> {
        if self.get_thread(thread_id)?.is_none() && self.load_thread(thread_id).await?.is_none() {
            return Err(SessionError::ThreadNotFound(thread_id));
        }

        let thread = self.threads.get(&thread_id).unwrap();
        Ok(thread.subscribe())
    }

    /// Get a thread aggregate by ID.
    /// Returns the thread with its event channel if found in memory or database.
    /// The returned Arc<Thread> shares ownership with Session's internal state.
    pub fn get_thread(&self, thread_id: Uuid) -> SessionResult<Option<Arc<Thread>>> {
        // Lazy-load thread from DB if not in memory
        if !self.threads.contains_key(&thread_id) {
            // Note: This is a sync function, so we can't do async DB calls here.
            // If the thread is not in memory, return None.
            // Callers should use load_thread() for lazy-loading from DB.
            return Ok(None);
        }

        Ok(self.threads.get(&thread_id).cloned())
    }

    /// Load a thread from database and add to session.
    /// Returns the thread aggregate if found.
    pub async fn load_thread(&mut self, thread_id: Uuid) -> SessionResult<Option<Arc<Thread>>> {
        if self.threads.contains_key(&thread_id) {
            return Ok(self.threads.get(&thread_id).cloned());
        }

        let Some(record) = self.load_owned_thread_record(thread_id).await? else {
            return Ok(None);
        };

        let deps = self.deps.clone();
        let arc = Arc::new(Self::thread_from_record(
            record,
            self.session_id.clone(),
            Arc::clone(&self.db),
            deps,
        ));
        self.threads.insert(thread_id, arc.clone());
        Ok(Some(arc))
    }

    async fn load_owned_thread_record(
        &self,
        thread_id: Uuid,
    ) -> SessionResult<Option<ThreadRecord>> {
        let Some(record) = self.db.get_thread(thread_id).await? else {
            return Ok(None)
        };

        if record.session_id != self.session_id {
            return Ok(None);
        }

        Ok(Some(record))
    }

    fn thread_from_record(
        record: ThreadRecord,
        session_id: String,
        db: DynSessionDatabase,
        deps: Option<Arc<TurnDependencies>>,
    ) -> Thread {
        let (event_tx, _) = broadcast::channel(64);
        Thread::new(record, session_id, event_tx, db, deps, None)
    }
}

// Helper structs and functions for turn lifecycle

#[derive(Debug, Clone)]
struct IncrementalTranscript {
    messages: Vec<PersistedMessage>,
    assistant_text: String,
    prepared_calls: Vec<crate::types::PersistedToolCall>,
    tool_results: std::collections::HashMap<String, IncrementalToolResult>,
}

#[derive(Debug, Clone)]
struct IncrementalToolResult {
    content: String,
    is_error: bool,
}

impl IncrementalTranscript {
    fn new(initial_messages: Vec<PersistedMessage>) -> Self {
        Self {
            messages: initial_messages,
            assistant_text: String::new(),
            prepared_calls: Vec::new(),
            tool_results: std::collections::HashMap::new(),
        }
    }

    fn apply_event(&mut self, event: &TurnEvent) {
        match event {
            TurnEvent::LlmTextDelta { text } => self.assistant_text.push_str(text.as_ref()),
            TurnEvent::ToolCallPrepared { call } => {
                self.prepared_calls.push(persist_tool_call(call.as_ref()));
            }
            TurnEvent::ToolCallCompleted { call_id, result } => {
                self.tool_results.insert(
                    call_id.as_ref().to_owned(),
                    IncrementalToolResult {
                        content: tool_outcome_content(result),
                        is_error: tool_outcome_is_error(result),
                    },
                );
            }
            TurnEvent::StepFinished { reason: turn::StepFinishReason::ToolCalls, .. } => {
                self.flush_pending_messages();
            }
            TurnEvent::TurnFinished { reason }
                if !matches!(reason, turn::TurnFinishReason::Cancelled) =>
            {
                self.flush_pending_messages();
            }
            _ => {}
        }
    }

    fn snapshot(&self) -> Vec<PersistedMessage> {
        let mut snapshot = self.clone();
        snapshot.flush_pending_messages();
        snapshot.messages
    }

    fn flush_pending_messages(&mut self) {
        if !self.prepared_calls.is_empty() {
            let calls = std::mem::take(&mut self.prepared_calls);
            let tool_results = std::mem::take(&mut self.tool_results);
            let content = (!self.assistant_text.is_empty())
                .then(|| std::mem::take(&mut self.assistant_text));
            self.messages.push(PersistedMessage::AssistantToolCalls {
                content,
                calls: calls.clone(),
            });

            for call in calls {
                if let Some(result) = tool_results.get(&call.call_id) {
                    self.messages.push(PersistedMessage::ToolResult {
                        call_id: call.call_id.clone(),
                        tool_name: call.tool_name.clone(),
                        content: result.content.clone(),
                        is_error: result.is_error,
                    });
                }
            }
            return;
        }

        if !self.assistant_text.is_empty() {
            self.messages.push(PersistedMessage::AssistantText {
                content: std::mem::take(&mut self.assistant_text),
            });
        }
    }
}

fn tool_outcome_content(outcome: &turn::ToolOutcome) -> String {
    match outcome {
        turn::ToolOutcome::Success(value) => value.to_string(),
        turn::ToolOutcome::Failed { message, .. } => message.as_ref().to_owned(),
        turn::ToolOutcome::TimedOut => "tool timed out".into(),
        turn::ToolOutcome::Denied => "tool call denied".into(),
        turn::ToolOutcome::Cancelled => "tool call cancelled".into(),
    }
}

fn tool_outcome_is_error(outcome: &turn::ToolOutcome) -> bool {
    !matches!(outcome, turn::ToolOutcome::Success(_))
}

fn apply_turn_outcome(
    turn_record: &mut TurnRecord,
    prior_message_count: usize,
    outcome: TurnOutcome,
) {
    turn_record.status = map_finish_reason_to_status(&outcome.finish_reason);
    turn_record.finish_reason = Some(format!("{:?}", outcome.finish_reason));
    let new_messages = outcome
        .transcript
        .get(prior_message_count..)
        .unwrap_or(&outcome.transcript);
    turn_record.transcript = persist_transcript(new_messages);
    turn_record.final_output = outcome.final_output;
    turn_record.finished_at = Some(Utc::now());
}

fn map_finish_reason_to_status(reason: &turn::TurnFinishReason) -> TurnStatus {
    match reason {
        turn::TurnFinishReason::Completed => TurnStatus::Completed,
        turn::TurnFinishReason::Cancelled => TurnStatus::Cancelled,
        turn::TurnFinishReason::Failed
        | turn::TurnFinishReason::MaxStepsExceeded
        | turn::TurnFinishReason::ModelLengthLimit
        | turn::TurnFinishReason::ModelProtocolError
        | turn::TurnFinishReason::LlmTimeout => TurnStatus::Failed,
    }
}
