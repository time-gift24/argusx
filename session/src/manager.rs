use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use tokio::{sync::broadcast, task::JoinHandle};
use turn::{
    ModelRunner, PermissionDecision, ToolAuthorizer, ToolRunner, TurnDriver, TurnEvent,
    TurnFinishReason, TurnHandle, TurnObserver, TurnOutcome, TurnSeed,
};
use uuid::Uuid;

use crate::{
    store::ThreadStore,
    thread::{ActiveTurnRuntime, ThreadRuntime, persist_transcript},
    types::{
        PersistedMessage, SessionRecord, ThreadEvent, ThreadLifecycle, ThreadRecord, TurnRecord,
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
        self.runtime.lock().unwrap().active_thread_id
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.events_tx.subscribe()
    }

    pub async fn initialize(&self) -> Result<u64> {
        let interrupted = self.store.mark_incomplete_turns_interrupted().await?;
        let mut runtime = self.runtime.lock().unwrap();
        runtime.active_thread_id = None;
        runtime.threads.clear();
        Ok(interrupted)
    }

    pub async fn create_thread(&self, title: Option<String>) -> Result<Uuid> {
        self.ensure_session().await?;

        let now = Utc::now();
        let thread = ThreadRecord {
            id: Uuid::new_v4(),
            session_id: self.session_id.clone(),
            title,
            lifecycle: ThreadLifecycle::Open,
            created_at: now,
            updated_at: now,
            last_turn_number: 0,
        };
        self.store.insert_thread(&thread).await?;

        {
            let mut runtime = self.runtime.lock().unwrap();
            runtime
                .threads
                .entry(thread.id)
                .or_insert_with(|| ThreadRuntime::new(thread.id));
            runtime.active_thread_id = Some(thread.id);
        }

        self.emit_thread_event(thread.id, ThreadEvent::ThreadCreated);
        self.emit_thread_event(thread.id, ThreadEvent::ThreadActivated);
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
            let mut runtime = self.runtime.lock().unwrap();
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

    pub async fn load_thread_history(&self, thread_id: Uuid) -> Result<Vec<TurnRecord>> {
        self.store.list_turns(thread_id).await
    }

    pub async fn send_message(
        &self,
        thread_id: Uuid,
        content: String,
        deps: TurnDependencies,
    ) -> Result<Uuid> {
        let mut thread = self
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

        let history = self.store.list_turns(thread_id).await?;
        let (turn_id, turn_number, prior_messages, prior_message_count) = {
            let mut runtime = self.runtime.lock().unwrap();
            let thread_runtime = runtime
                .threads
                .entry(thread_id)
                .or_insert_with(|| ThreadRuntime::new(thread_id));
            if thread_runtime.active_turn.is_some() {
                bail!("thread {thread_id} already has an active turn");
            }

            let turn_number = thread.last_turn_number + 1;
            let turn_id = Uuid::new_v4();
            let prior_messages = thread_runtime.build_prior_messages(&history);
            let prior_message_count = prior_messages.len();
            (turn_id, turn_number, prior_messages, prior_message_count)
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
        self.store.insert_turn(&turn_record).await?;

        thread.last_turn_number = turn_number;
        thread.updated_at = now;
        self.store.update_thread(&thread).await?;

        let seed = TurnSeed {
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            prior_messages,
            user_message: content,
        };
        let (handle, task) = TurnDriver::spawn(
            seed,
            deps.model,
            deps.tool_runner,
            deps.authorizer,
            deps.observer,
        );

        {
            let mut runtime = self.runtime.lock().unwrap();
            let thread_runtime = runtime
                .threads
                .entry(thread_id)
                .or_insert_with(|| ThreadRuntime::new(thread_id));
            thread_runtime.active_turn = Some(ActiveTurnRuntime {
                turn_id,
                turn_number,
                controller: handle.controller(),
                waiting_permission: None,
            });
        }

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
            while let Some(event) = handle.next_event().await {
                match &event {
                    TurnEvent::ToolCallPermissionRequested { request } => {
                        turn_record.status = TurnStatus::WaitingPermission;
                        {
                            let mut runtime = runtime.lock().unwrap();
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
                        {
                            let mut runtime = runtime.lock().unwrap();
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
                    turn_record.status = TurnStatus::Failed;
                    turn_record.finish_reason = Some("Failed".into());
                    turn_record.finished_at = Some(Utc::now());
                    let _ = store.update_turn(&turn_record).await;
                }
                Err(_join_err) => {
                    turn_record.status = TurnStatus::Failed;
                    turn_record.finish_reason = Some("Failed".into());
                    turn_record.finished_at = Some(Utc::now());
                    let _ = store.update_turn(&turn_record).await;
                }
            }

            {
                let mut runtime = runtime.lock().unwrap();
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
        self.runtime
            .lock()
            .unwrap()
            .threads
            .get(&thread_id)
            .and_then(|thread| thread.active_turn.as_ref())
            .map(|turn| turn.controller.clone())
            .with_context(|| format!("thread {thread_id} does not have an active turn"))
    }

    fn emit_thread_event(&self, thread_id: Uuid, event: ThreadEvent) {
        let _ = self
            .events_tx
            .send(SessionEvent::Thread { thread_id, event });
    }

    async fn ensure_session(&self) -> Result<()> {
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
}
