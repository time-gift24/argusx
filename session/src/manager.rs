use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use tokio::{sync::broadcast, task::JoinHandle};
use turn::{
    ModelRunner, PermissionDecision, StepFinishReason, ToolAuthorizer, ToolOutcome, ToolRunner,
    TurnDriver, TurnEvent, TurnFinishReason, TurnHandle, TurnObserver, TurnOutcome, TurnSeed,
};
use uuid::Uuid;

use crate::{
    store::ThreadStore,
    thread::{ActiveTurnRuntime, ThreadRuntime, persist_tool_call, persist_transcript},
    types::{
        PersistedMessage, PersistedToolCall, SessionRecord, ThreadAgentSnapshotRecord,
        ThreadAgentSnapshotSeed, ThreadEvent, ThreadLifecycle, ThreadRecord, TurnRecord,
        TurnStatus,
    },
};

const DEFAULT_MODEL: &str = "gpt-5";

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Thread {
        thread_id: Uuid,
        event: ThreadEvent,
    },
    Turn {
        thread_id: Uuid,
        turn_id: Uuid,
        event: TurnEvent,
    },
}

#[derive(Clone)]
pub struct TurnDependencies {
    pub system_prompt: Option<String>,
    pub model: Arc<dyn ModelRunner>,
    pub tool_runner: Arc<dyn ToolRunner>,
    pub authorizer: Arc<dyn ToolAuthorizer>,
    pub observer: Arc<dyn TurnObserver>,
}

#[derive(Clone)]
pub struct SessionManager {
    session_id: String,
    store: ThreadStore,
    runtime: Arc<Mutex<SessionRuntime>>,
    events_tx: broadcast::Sender<SessionEvent>,
}

impl SessionManager {
    pub fn new(session_id: String, store: ThreadStore) -> Self {
        let (events_tx, _) = broadcast::channel(64);
        Self {
            session_id,
            store,
            runtime: Arc::new(Mutex::new(SessionRuntime::default())),
            events_tx,
        }
    }

    pub fn active_thread_id(&self) -> Option<Uuid> {
        self.lock_runtime().active_thread_id
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.events_tx.subscribe()
    }

    pub async fn initialize(&self) -> Result<u64> {
        let interrupted = self.store.mark_incomplete_turns_interrupted().await?;
        let mut runtime = self.lock_runtime();
        runtime.active_thread_id = None;
        runtime.threads.clear();
        Ok(interrupted)
    }

    pub async fn create_thread(&self, title: Option<String>) -> Result<Uuid> {
        self.create_thread_with_binding(title, None, false, true, None)
            .await
    }

    pub async fn create_thread_with_agent_binding(
        &self,
        title: Option<String>,
        snapshot: ThreadAgentSnapshotSeed,
        is_subagent: bool,
        activate: bool,
    ) -> Result<Uuid> {
        let agent_profile_id = snapshot.profile_id.clone();
        self.create_thread_with_binding(
            title,
            Some(agent_profile_id),
            is_subagent,
            activate,
            Some(snapshot),
        )
        .await
    }

    async fn create_thread_with_binding(
        &self,
        title: Option<String>,
        agent_profile_id: Option<String>,
        is_subagent: bool,
        activate: bool,
        snapshot: Option<ThreadAgentSnapshotSeed>,
    ) -> Result<Uuid> {
        self.ensure_session().await?;

        let now = Utc::now();
        let thread = ThreadRecord {
            id: Uuid::new_v4(),
            session_id: self.session_id.clone(),
            agent_profile_id,
            is_subagent,
            title,
            lifecycle: ThreadLifecycle::Open,
            created_at: now,
            updated_at: now,
            last_turn_number: 0,
        };
        self.store.insert_thread(&thread).await?;
        if let Some(snapshot) = snapshot {
            self.store
                .insert_thread_agent_snapshot(&ThreadAgentSnapshotRecord {
                    thread_id: thread.id,
                    profile_id: snapshot.profile_id,
                    display_name_snapshot: snapshot.display_name_snapshot,
                    system_prompt_snapshot: snapshot.system_prompt_snapshot,
                    tool_policy_snapshot_json: snapshot.tool_policy_snapshot_json,
                    model_config_snapshot_json: snapshot.model_config_snapshot_json,
                    allow_subagent_dispatch_snapshot: snapshot.allow_subagent_dispatch_snapshot,
                    created_at: now,
                })
                .await?;
        }

        {
            let mut runtime = self.lock_runtime();
            runtime
                .threads
                .entry(thread.id)
                .or_insert_with(|| ThreadRuntime::new(thread.id));
            if activate {
                runtime.active_thread_id = Some(thread.id);
            }
        }

        self.emit_thread_event(thread.id, ThreadEvent::ThreadCreated);
        if activate {
            self.emit_thread_event(thread.id, ThreadEvent::ThreadActivated);
        }
        Ok(thread.id)
    }

    pub async fn switch_thread(&self, thread_id: Uuid) -> Result<()> {
        let thread = self
            .store
            .get_thread(thread_id)
            .await?
            .with_context(|| format!("thread not found: {thread_id}"))?;
        if thread.session_id != self.session_id {
            bail!(
                "thread {thread_id} does not belong to session {}",
                self.session_id
            );
        }

        {
            let mut runtime = self.lock_runtime();
            runtime
                .threads
                .entry(thread_id)
                .or_insert_with(|| ThreadRuntime::new(thread_id));
            runtime.active_thread_id = Some(thread_id);
        }

        self.emit_thread_event(thread_id, ThreadEvent::ThreadActivated);
        Ok(())
    }

    pub async fn list_threads(&self) -> Result<Vec<ThreadRecord>> {
        self.store.list_threads(&self.session_id).await
    }

    pub async fn load_session(&self) -> Result<Option<SessionRecord>> {
        self.store.get_session(&self.session_id).await
    }

    pub async fn load_thread(&self, thread_id: Uuid) -> Result<Option<ThreadRecord>> {
        let thread = self.store.get_thread(thread_id).await?;
        match thread {
            Some(thread) if thread.session_id == self.session_id => Ok(Some(thread)),
            Some(_thread) => bail!(
                "thread {thread_id} does not belong to session {}",
                self.session_id
            ),
            None => Ok(None),
        }
    }

    pub async fn load_thread_history(&self, thread_id: Uuid) -> Result<Vec<TurnRecord>> {
        self.store.list_turns(thread_id).await
    }

    pub async fn load_thread_agent_snapshot(
        &self,
        thread_id: Uuid,
    ) -> Result<Option<ThreadAgentSnapshotRecord>> {
        let thread = self
            .store
            .get_thread(thread_id)
            .await?
            .with_context(|| format!("thread not found: {thread_id}"))?;
        if thread.session_id != self.session_id {
            bail!(
                "thread {thread_id} does not belong to session {}",
                self.session_id
            );
        }

        self.store.get_thread_agent_snapshot(thread_id).await
    }

    pub async fn send_message(
        &self,
        thread_id: Uuid,
        content: String,
        deps: TurnDependencies,
    ) -> Result<Uuid> {
        self.send_message_with_turn_id(thread_id, Uuid::new_v4(), content, deps)
            .await
    }

    pub async fn send_message_with_turn_id(
        &self,
        thread_id: Uuid,
        turn_id: Uuid,
        content: String,
        deps: TurnDependencies,
    ) -> Result<Uuid> {
        let thread = self
            .store
            .get_thread(thread_id)
            .await?
            .with_context(|| format!("thread not found: {thread_id}"))?;
        if thread.session_id != self.session_id {
            bail!(
                "thread {thread_id} does not belong to session {}",
                self.session_id
            );
        }

        // 先在内存里占坑，保证同一 thread 的 send_message 不会跨过 await 并发启动两个 turn。
        let mut reservation =
            ActiveTurnReservation::reserve(Arc::clone(&self.runtime), thread_id, turn_id)?;

        let history = self.store.list_turns(thread_id).await?;
        let thread = self
            .store
            .get_thread(thread_id)
            .await?
            .with_context(|| format!("thread not found: {thread_id}"))?;
        if thread.session_id != self.session_id {
            bail!(
                "thread {thread_id} does not belong to session {}",
                self.session_id
            );
        }

        let (turn_number, prior_messages, prior_message_count) = {
            let mut runtime = self.lock_runtime();
            let thread_runtime = runtime
                .threads
                .entry(thread_id)
                .or_insert_with(|| ThreadRuntime::new(thread_id));
            let turn_number = thread.last_turn_number + 1;
            let prior_messages = thread_runtime.build_prior_messages(&history);
            let prior_message_count = prior_messages.len();
            (turn_number, prior_messages, prior_message_count)
        };

        let now = Utc::now();
        let turn_record = TurnRecord {
            id: turn_id,
            thread_id,
            turn_number,
            user_input: content.clone(),
            status: TurnStatus::Running,
            finish_reason: None,
            transcript: vec![PersistedMessage::User {
                content: content.clone(),
            }],
            final_output: None,
            started_at: now,
            finished_at: None,
        };
        self.store
            .insert_turn_and_advance_thread(&turn_record, thread.last_turn_number, now)
            .await?;

        let seed = TurnSeed {
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            prior_messages,
            user_message: content,
            system_prompt: deps.system_prompt,
        };
        let (handle, task) = TurnDriver::spawn(
            seed,
            deps.model,
            deps.tool_runner,
            deps.authorizer,
            deps.observer,
        );

        reservation.activate(turn_number, handle.controller())?;

        self.spawn_turn_bridge(
            thread_id,
            turn_record.clone(),
            prior_message_count,
            handle,
            task,
        );
        Ok(turn_id)
    }

    pub async fn resolve_permission(
        &self,
        thread_id: Uuid,
        request_id: String,
        decision: PermissionDecision,
    ) -> Result<()> {
        let controller = self.active_turn_controller(thread_id)?;
        controller.resolve_permission(request_id, decision).await?;
        Ok(())
    }

    pub async fn cancel_turn(&self, thread_id: Uuid) -> Result<()> {
        let controller = self.active_turn_controller(thread_id)?;
        controller.cancel().await?;
        Ok(())
    }

    fn spawn_turn_bridge(
        &self,
        thread_id: Uuid,
        mut turn_record: TurnRecord,
        prior_message_count: usize,
        handle: TurnHandle,
        task: JoinHandle<Result<TurnOutcome, turn::TurnError>>,
    ) {
        let runtime = Arc::clone(&self.runtime);
        let store = self.store.clone();
        let events_tx = self.events_tx.clone();

        tokio::spawn(async move {
            let mut transcript = IncrementalTranscript::new(turn_record.transcript.clone());

            while let Some(event) = handle.next_event().await {
                transcript.apply_event(&event);

                match &event {
                    TurnEvent::ToolCallPermissionRequested { request } => {
                        turn_record.status = TurnStatus::WaitingPermission;
                        turn_record.transcript = transcript.snapshot();
                        {
                            let mut runtime = lock_session_runtime(&runtime);
                            if let Some(thread_runtime) = runtime.threads.get_mut(&thread_id)
                                && let Some(active_turn) = thread_runtime.active_turn.as_mut()
                                && active_turn.turn_id == turn_record.id
                            {
                                active_turn.waiting_permission = Some(request.clone());
                            }
                        }
                        let _ = store.update_turn(&turn_record).await;
                    }
                    TurnEvent::ToolCallPermissionResolved { .. } => {
                        turn_record.status = TurnStatus::Running;
                        turn_record.transcript = transcript.snapshot();
                        {
                            let mut runtime = lock_session_runtime(&runtime);
                            if let Some(thread_runtime) = runtime.threads.get_mut(&thread_id)
                                && let Some(active_turn) = thread_runtime.active_turn.as_mut()
                                && active_turn.turn_id == turn_record.id
                            {
                                active_turn.waiting_permission = None;
                            }
                        }
                        let _ = store.update_turn(&turn_record).await;
                    }
                    _ => {}
                }

                let _ = events_tx.send(SessionEvent::Turn {
                    thread_id,
                    turn_id: turn_record.id,
                    event: event.clone(),
                });
            }

            match task.await {
                Ok(Ok(outcome)) => {
                    apply_turn_outcome(&mut turn_record, prior_message_count, outcome);
                    let _ = store.update_turn(&turn_record).await;
                }
                Ok(Err(_turn_err)) => {
                    // 失败分支拿不到 TurnOutcome，需要把已发给 UI 的增量事件补写回 transcript。
                    turn_record.status = TurnStatus::Failed;
                    turn_record.finish_reason = Some("Failed".into());
                    turn_record.transcript = transcript.snapshot();
                    turn_record.finished_at = Some(Utc::now());
                    let _ = store.update_turn(&turn_record).await;
                }
                Err(_join_err) => {
                    turn_record.status = TurnStatus::Failed;
                    turn_record.finish_reason = Some("Failed".into());
                    turn_record.transcript = transcript.snapshot();
                    turn_record.finished_at = Some(Utc::now());
                    let _ = store.update_turn(&turn_record).await;
                }
            }

            {
                let mut runtime = lock_session_runtime(&runtime);
                if let Some(thread_runtime) = runtime.threads.get_mut(&thread_id)
                    && thread_runtime
                        .active_turn
                        .as_ref()
                        .map(|active| active.turn_id == turn_record.id)
                        .unwrap_or(false)
                {
                    thread_runtime.active_turn = None;
                }
            }

            let _ = events_tx.send(SessionEvent::Thread {
                thread_id,
                event: ThreadEvent::ThreadUpdated,
            });
        });
    }

    fn active_turn_controller(&self, thread_id: Uuid) -> Result<turn::TurnController> {
        let runtime = self.lock_runtime();
        let active_turn = runtime
            .threads
            .get(&thread_id)
            .and_then(|thread| thread.active_turn.as_ref())
            .with_context(|| format!("thread {thread_id} does not have an active turn"))?;

        active_turn.controller.clone().with_context(|| {
            format!("thread {thread_id} has an active turn that is still starting")
        })
    }

    fn emit_thread_event(&self, thread_id: Uuid, event: ThreadEvent) {
        let _ = self
            .events_tx
            .send(SessionEvent::Thread { thread_id, event });
    }

    fn lock_runtime(&self) -> MutexGuard<'_, SessionRuntime> {
        lock_session_runtime(&self.runtime)
    }

    async fn ensure_session(&self) -> Result<()> {
        if self.store.get_session(&self.session_id).await?.is_some() {
            return Ok(());
        }

        let now = Utc::now();
        self.store
            .upsert_session(&SessionRecord {
                id: self.session_id.clone(),
                user_id: None,
                default_model: DEFAULT_MODEL.into(),
                system_prompt: None,
                created_at: now,
                updated_at: now,
            })
            .await
    }
}

#[derive(Debug, Default)]
pub struct SessionRuntime {
    pub active_thread_id: Option<Uuid>,
    pub threads: HashMap<Uuid, ThreadRuntime>,
}

fn lock_session_runtime(runtime: &Arc<Mutex<SessionRuntime>>) -> MutexGuard<'_, SessionRuntime> {
    // SessionRuntime 只有派生性的内存态，真正的稳定真相在 store 里；锁被 poison 时优先恢复而不是把后续所有调用都 panic 掉。
    runtime.lock().unwrap_or_else(|poisoned| {
        eprintln!("session runtime mutex poisoned, recovering in-memory state");
        poisoned.into_inner()
    })
}

struct ActiveTurnReservation {
    runtime: Arc<Mutex<SessionRuntime>>,
    thread_id: Uuid,
    turn_id: Uuid,
    committed: bool,
}

impl ActiveTurnReservation {
    fn reserve(
        runtime: Arc<Mutex<SessionRuntime>>,
        thread_id: Uuid,
        turn_id: Uuid,
    ) -> Result<Self> {
        {
            let mut locked = lock_session_runtime(&runtime);
            let thread_runtime = locked
                .threads
                .entry(thread_id)
                .or_insert_with(|| ThreadRuntime::new(thread_id));
            if thread_runtime.active_turn.is_some() {
                bail!("thread {thread_id} already has an active turn");
            }
            thread_runtime.active_turn = Some(ActiveTurnRuntime::starting(turn_id));
        }

        Ok(Self {
            runtime,
            thread_id,
            turn_id,
            committed: false,
        })
    }

    fn activate(&mut self, turn_number: u32, controller: turn::TurnController) -> Result<()> {
        let mut runtime = lock_session_runtime(&self.runtime);
        let active_turn = runtime
            .threads
            .get_mut(&self.thread_id)
            .and_then(|thread| thread.active_turn.as_mut())
            .filter(|turn| turn.turn_id == self.turn_id)
            .with_context(|| {
                format!(
                    "thread {} lost active turn reservation during startup",
                    self.thread_id
                )
            })?;

        active_turn.activate(turn_number, controller);
        self.committed = true;
        Ok(())
    }
}

impl Drop for ActiveTurnReservation {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        let mut runtime = lock_session_runtime(&self.runtime);
        if let Some(thread_runtime) = runtime.threads.get_mut(&self.thread_id)
            && thread_runtime
                .active_turn
                .as_ref()
                .map(|turn| turn.turn_id == self.turn_id)
                .unwrap_or(false)
        {
            thread_runtime.active_turn = None;
        }
    }
}

#[derive(Debug, Clone)]
struct IncrementalTranscript {
    messages: Vec<PersistedMessage>,
    assistant_text: String,
    prepared_calls: Vec<PersistedToolCall>,
    tool_results: HashMap<String, IncrementalToolResult>,
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
            tool_results: HashMap::new(),
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
            TurnEvent::StepFinished {
                reason: StepFinishReason::ToolCalls,
                ..
            } => self.flush_pending_messages(),
            TurnEvent::TurnFinished { reason }
                if !matches!(reason, TurnFinishReason::Cancelled) =>
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
            let content =
                (!self.assistant_text.is_empty()).then(|| std::mem::take(&mut self.assistant_text));
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

fn tool_outcome_content(outcome: &ToolOutcome) -> String {
    match outcome {
        ToolOutcome::Success(value) => value.to_string(),
        ToolOutcome::Failed { message, .. } => message.as_ref().to_owned(),
        ToolOutcome::TimedOut => "tool timed out".into(),
        ToolOutcome::Denied => "tool call denied".into(),
        ToolOutcome::Cancelled => "tool call cancelled".into(),
    }
}

fn tool_outcome_is_error(outcome: &ToolOutcome) -> bool {
    !matches!(outcome, ToolOutcome::Success(_))
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

fn map_finish_reason_to_status(reason: &TurnFinishReason) -> TurnStatus {
    match reason {
        TurnFinishReason::Completed => TurnStatus::Completed,
        TurnFinishReason::Cancelled => TurnStatus::Cancelled,
        TurnFinishReason::Failed
        | TurnFinishReason::MaxStepsExceeded
        | TurnFinishReason::ModelLengthLimit
        | TurnFinishReason::ModelProtocolError
        | TurnFinishReason::LlmTimeout => TurnStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn map_finish_reason_completed_to_completed_status() {
        assert_eq!(
            map_finish_reason_to_status(&TurnFinishReason::Completed),
            TurnStatus::Completed
        );
        assert_eq!(
            map_finish_reason_to_status(&TurnFinishReason::Cancelled),
            TurnStatus::Cancelled
        );
        assert_eq!(
            map_finish_reason_to_status(&TurnFinishReason::LlmTimeout),
            TurnStatus::Failed
        );
    }

    #[test]
    fn lock_session_runtime_recovers_from_poisoned_mutex() {
        let runtime = Arc::new(Mutex::new(SessionRuntime::default()));
        let poison_runtime = Arc::clone(&runtime);

        let _ = std::panic::catch_unwind(move || {
            let _guard = poison_runtime.lock().unwrap();
            panic!("poison session runtime");
        });

        assert!(runtime.is_poisoned());

        let guard = lock_session_runtime(&runtime);
        assert!(guard.active_thread_id.is_none());
    }
}
