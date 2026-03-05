use std::collections::HashMap;
use std::sync::Arc;

use agent_core::{
    new_id, AgentError, CheckpointStore, InputEnvelope, Runtime, RuntimeError, RuntimeEvent,
    RuntimeStreams, TurnRequest,
};
use async_trait::async_trait;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::bus::{BusConfig, EventBus};
use crate::command::normalizer::CommandNormalizer;
use crate::effect::EffectExecutor;
use crate::engine::TurnEngine;
use crate::handlers::HandlerRegistry;
use crate::journal::TranscriptJournal;
use crate::state::{TurnEngineConfig, TurnState};

#[derive(Clone)]
struct TurnControl {
    event_tx: mpsc::UnboundedSender<RuntimeEvent>,
}

pub struct TurnRuntime<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + 'static,
{
    model: Arc<L>,
    tools: Arc<T>,
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
    config: TurnEngineConfig,
    turns: Arc<RwLock<HashMap<String, TurnControl>>>,
}

impl<L, T> TurnRuntime<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + 'static,
{
    pub fn new(model: Arc<L>, tools: Arc<T>, config: TurnEngineConfig) -> Self {
        Self {
            model,
            tools,
            checkpoint_store: None,
            config,
            turns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_checkpoint_store(mut self, store: Arc<dyn CheckpointStore>) -> Self {
        self.checkpoint_store = Some(store);
        self
    }
}

#[async_trait]
impl<L, T> Runtime for TurnRuntime<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + 'static,
{
    async fn run_turn(&self, request: TurnRequest) -> Result<RuntimeStreams, AgentError> {
        let TurnRequest {
            meta,
            provider,
            model,
            initial_input,
            transcript,
        } = request;

        let turn_id = meta.turn_id.clone();
        {
            let turns = self.turns.read().await;
            if turns.contains_key(&turn_id) {
                return Err(RuntimeError::TurnAlreadyExists { turn_id }.into());
            }
        }

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (run_tx, run_rx) = mpsc::unbounded_channel();
        let (ui_tx, ui_rx) = mpsc::unbounded_channel();

        let mut state = TurnState::new(meta.clone(), provider, model);
        state.transcript = transcript;
        let journal = TranscriptJournal::default();

        let effect_executor = EffectExecutor::new(
            Arc::clone(&self.model),
            Arc::clone(&self.tools),
            event_tx.clone(),
            self.config.max_parallel_tools,
        );

        let engine = TurnEngine {
            config: self.config.clone(),
            state,
            journal: journal.clone(),
            effect_executor,
            event_rx,
            run_tx,
            ui_tx,
            checkpoint_store: self.checkpoint_store.clone(),
            bus: EventBus::new(BusConfig::default()),
            normalizer: CommandNormalizer::default(),
            handlers: HandlerRegistry::default(),
        };

        {
            let mut turns = self.turns.write().await;
            turns.insert(
                turn_id.clone(),
                TurnControl {
                    event_tx: event_tx.clone(),
                },
            );
        }

        let turns = Arc::clone(&self.turns);
        let checkpoint = self.checkpoint_store.clone();
        tokio::spawn(async move {
            let final_state = engine.run().await;
            if let Some(store) = checkpoint {
                let items = journal.all().await;
                let _ = store.snapshot(&final_state.meta.turn_id, &items).await;
            }
            let mut guard = turns.write().await;
            guard.remove(&final_state.meta.turn_id);
        });

        if event_tx
            .send(RuntimeEvent::TurnStarted {
                event_id: new_id(),
                turn_id: meta.turn_id,
                input: initial_input,
            })
            .is_err()
        {
            return Err(AgentError::Internal {
                message: "failed to start turn loop".to_string(),
            });
        }

        Ok(RuntimeStreams {
            run: Box::pin(UnboundedReceiverStream::new(run_rx)),
            ui: Box::pin(UnboundedReceiverStream::new(ui_rx)),
        })
    }

    async fn inject_input(&self, turn_id: &str, input: InputEnvelope) -> Result<(), AgentError> {
        let turns = self.turns.read().await;
        let control = turns
            .get(turn_id)
            .ok_or_else(|| RuntimeError::TurnNotFound {
                turn_id: turn_id.to_string(),
            })?;

        control
            .event_tx
            .send(RuntimeEvent::InputInjected {
                event_id: new_id(),
                input,
            })
            .map_err(|_| AgentError::Internal {
                message: format!("failed to inject input to turn {turn_id}"),
            })
    }

    async fn cancel_turn(&self, turn_id: &str, reason: Option<String>) -> Result<(), AgentError> {
        let turns = self.turns.read().await;
        let control = turns
            .get(turn_id)
            .ok_or_else(|| RuntimeError::TurnNotFound {
                turn_id: turn_id.to_string(),
            })?;

        control
            .event_tx
            .send(RuntimeEvent::CancelRequested {
                event_id: new_id(),
                reason,
            })
            .map_err(|_| AgentError::Internal {
                message: format!("failed to cancel turn {turn_id}"),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agent_core::tools::{
        ToolCatalog, ToolExecutionContext, ToolExecutionError, ToolExecutor, ToolSpec,
    };
    use agent_core::{
        AgentError, CheckpointStore, InputEnvelope, LanguageModel, ModelEventStream,
        ModelOutputEvent, RunStreamEvent, Runtime, RuntimeEvent, SessionMeta, TranscriptItem,
        TurnRequest,
    };
    use async_trait::async_trait;
    use futures::{stream, StreamExt};
    use tokio::sync::Mutex;

    use super::TurnRuntime;
    use crate::reducer::reduce;
    use crate::state::TurnEngineConfig;

    struct InstantDoneModel;
    struct TextThenDoneModel;

    #[async_trait]
    impl LanguageModel for InstantDoneModel {
        fn model_name(&self) -> &str {
            "instant-done"
        }

        async fn stream(
            &self,
            _request: agent_core::ModelRequest,
        ) -> Result<ModelEventStream, AgentError> {
            Ok(Box::pin(stream::once(async {
                Ok(ModelOutputEvent::Completed { usage: None })
            })))
        }
    }

    #[async_trait]
    impl LanguageModel for TextThenDoneModel {
        fn model_name(&self) -> &str {
            "text-then-done"
        }

        async fn stream(
            &self,
            _request: agent_core::ModelRequest,
        ) -> Result<ModelEventStream, AgentError> {
            Ok(Box::pin(stream::iter(vec![
                Ok(ModelOutputEvent::TextDelta {
                    delta: "hello".to_string(),
                }),
                Ok(ModelOutputEvent::Completed { usage: None }),
            ])))
        }
    }

    struct NoopTools;

    #[async_trait]
    impl ToolCatalog for NoopTools {
        async fn list_tools(&self) -> Vec<ToolSpec> {
            Vec::new()
        }

        async fn tool_spec(&self, _name: &str) -> Option<ToolSpec> {
            None
        }
    }

    #[async_trait]
    impl ToolExecutor for NoopTools {
        async fn execute_tool(
            &self,
            call: agent_core::ToolCall,
            _ctx: ToolExecutionContext,
        ) -> Result<agent_core::ToolResult, ToolExecutionError> {
            Ok(agent_core::ToolResult::ok(
                call.call_id,
                serde_json::json!({}),
            ))
        }
    }

    #[derive(Default)]
    struct RecordingCheckpointStore {
        append_calls: Mutex<Vec<(String, Vec<TranscriptItem>)>>,
        snapshots: Mutex<Vec<(String, Vec<TranscriptItem>)>>,
    }

    #[async_trait]
    impl CheckpointStore for RecordingCheckpointStore {
        async fn append_items(
            &self,
            turn_id: &str,
            items: &[TranscriptItem],
        ) -> Result<(), AgentError> {
            self.append_calls
                .lock()
                .await
                .push((turn_id.to_string(), items.to_vec()));
            Ok(())
        }

        async fn load_items(&self, _turn_id: &str) -> Result<Vec<TranscriptItem>, AgentError> {
            Ok(Vec::new())
        }

        async fn snapshot(
            &self,
            turn_id: &str,
            items: &[TranscriptItem],
        ) -> Result<(), AgentError> {
            self.snapshots
                .lock()
                .await
                .push((turn_id.to_string(), items.to_vec()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn runtime_appends_transcript_items_to_checkpoint_store() {
        let checkpoint = Arc::new(RecordingCheckpointStore::default());
        let runtime = TurnRuntime::new(
            Arc::new(InstantDoneModel),
            Arc::new(NoopTools),
            TurnEngineConfig::default(),
        )
        .with_checkpoint_store(checkpoint.clone());

        let request = TurnRequest::new(
            SessionMeta::new("s1", "t1"),
            "provider",
            "model",
            InputEnvelope::user_text("hello"),
        );

        let streams = runtime.run_turn(request).await.expect("run turn");
        let mut run = streams.run;
        while let Some(event) = run.next().await {
            if matches!(
                event,
                RunStreamEvent::TurnDone { .. } | RunStreamEvent::TurnFailed { .. }
            ) {
                break;
            }
        }

        let append_calls = checkpoint.append_calls.lock().await.clone();
        assert!(
            !append_calls.is_empty(),
            "expected incremental checkpoint append calls"
        );
        assert_eq!(append_calls[0].0, "t1");
        assert!(
            matches!(
                append_calls[0].1.first(),
                Some(TranscriptItem::UserMessage { .. })
            ),
            "expected first appended transcript item to be user message"
        );
    }

    #[tokio::test]
    async fn runtime_routes_model_text_delta_through_bus_pipeline_when_enabled() {
        let runtime = TurnRuntime::new(
            Arc::new(TextThenDoneModel),
            Arc::new(NoopTools),
            TurnEngineConfig::default(),
        );

        let request = TurnRequest::new(
            SessionMeta::new("s1", "t1"),
            "provider",
            "model",
            InputEnvelope::user_text("hello"),
        );

        let streams = runtime.run_turn(request).await.expect("run turn");
        let mut ui = streams.ui;
        let mut message_deltas = Vec::new();
        while let Some(event) = ui.next().await {
            match event {
                agent_core::UiThreadEvent::MessageDelta { delta, .. } => {
                    message_deltas.push(delta);
                }
                agent_core::UiThreadEvent::Done { .. }
                | agent_core::UiThreadEvent::Error { .. } => {
                    break;
                }
                _ => {}
            }
        }

        assert_eq!(message_deltas, vec!["hello".to_string()]);
    }

    #[tokio::test]
    async fn legacy_reducer_equivalence_trace_matches_bus_trace_for_core_flow() {
        async fn run_bus_runtime_and_capture_final_message(turn_id: &str) -> Option<String> {
            let runtime = TurnRuntime::new(
                Arc::new(TextThenDoneModel),
                Arc::new(NoopTools),
                TurnEngineConfig::default(),
            );
            let request = TurnRequest::new(
                SessionMeta::new("s1", turn_id),
                "provider",
                "model",
                InputEnvelope::user_text("hello"),
            );
            let streams = runtime.run_turn(request).await.expect("run turn");
            let mut run = streams.run;
            while let Some(event) = run.next().await {
                match event {
                    RunStreamEvent::TurnDone { final_message, .. } => return final_message,
                    RunStreamEvent::TurnFailed { message, .. } => {
                        panic!("turn failed unexpectedly: {message}");
                    }
                    _ => {}
                }
            }
            None
        }

        fn run_legacy_reducer_trace_and_capture_final_message() -> Option<String> {
            let mut state = crate::state::TurnState::new(
                SessionMeta::new("s1", "t-legacy"),
                "provider",
                "model",
            );
            let cfg = TurnEngineConfig::default();
            let events = vec![
                RuntimeEvent::TurnStarted {
                    event_id: "e1".into(),
                    turn_id: "t-legacy".into(),
                    input: InputEnvelope::user_text("hello"),
                },
                RuntimeEvent::ModelTextDelta {
                    event_id: "e2".into(),
                    epoch: 0,
                    delta: "hello".into(),
                },
                RuntimeEvent::ModelCompleted {
                    event_id: "e3".into(),
                    epoch: 0,
                    usage: None,
                },
            ];

            let mut final_message = None;
            for event in events {
                let tr = reduce(state, event, &cfg);
                for run_event in &tr.run_events {
                    if let RunStreamEvent::TurnDone {
                        final_message: message,
                        ..
                    } = run_event
                    {
                        final_message = message.clone();
                    }
                }
                state = tr.state;
            }
            final_message
        }

        let legacy = run_legacy_reducer_trace_and_capture_final_message();
        let bus = run_bus_runtime_and_capture_final_message("t-bus").await;

        assert_eq!(legacy, bus);
        assert_eq!(bus, Some("hello".to_string()));
    }
}
