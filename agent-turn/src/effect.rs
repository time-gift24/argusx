use std::sync::Arc;
use std::time::Duration;

use agent_core::{
    tools::{ToolExecutionContext, ToolExecutor},
    new_id, AgentError, LanguageModel, ModelOutputEvent, ModelRequest, RuntimeEvent, ToolCall,
    ToolResult, TranscriptItem,
};
use futures::StreamExt;
use tokio::sync::{mpsc, Semaphore};

#[derive(Debug, Clone)]
pub enum Effect {
    StartModel {
        epoch: u64,
        transcript: Vec<TranscriptItem>,
        inputs: Vec<agent_core::InputEnvelope>,
    },
    ExecuteTool {
        epoch: u64,
        session_id: String,
        turn_id: String,
        call: ToolCall,
    },
    ScheduleRetry {
        delay_ms: u64,
        next_epoch: u64,
    },
    PersistCheckpoint,
    CancelInflightTools,
}

#[derive(Clone)]
pub struct EffectExecutor<L, T>
where
    L: LanguageModel + 'static,
    T: ToolExecutor + 'static,
{
    model: Arc<L>,
    tools: Arc<T>,
    tx: mpsc::UnboundedSender<RuntimeEvent>,
    semaphore: Arc<Semaphore>,
}

impl<L, T> EffectExecutor<L, T>
where
    L: LanguageModel + 'static,
    T: ToolExecutor + 'static,
{
    pub fn new(
        model: Arc<L>,
        tools: Arc<T>,
        tx: mpsc::UnboundedSender<RuntimeEvent>,
        max_parallel_tools: usize,
    ) -> Self {
        Self {
            model,
            tools,
            tx,
            semaphore: Arc::new(Semaphore::new(max_parallel_tools.max(1))),
        }
    }

    pub async fn execute(&self, effect: Effect) {
        match effect {
            Effect::StartModel {
                epoch,
                transcript,
                inputs,
            } => {
                self.spawn_model_stream(epoch, transcript, inputs);
            }
            Effect::ExecuteTool {
                epoch,
                session_id,
                turn_id,
                call,
            } => {
                self.spawn_tool_execution(epoch, session_id, turn_id, call);
            }
            Effect::ScheduleRetry {
                delay_ms,
                next_epoch,
            } => {
                self.spawn_retry_timer(delay_ms, next_epoch);
            }
            Effect::PersistCheckpoint | Effect::CancelInflightTools => {}
        }
    }

    fn spawn_model_stream(
        &self,
        epoch: u64,
        transcript: Vec<TranscriptItem>,
        inputs: Vec<agent_core::InputEnvelope>,
    ) {
        let model = Arc::clone(&self.model);
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let request = ModelRequest {
                epoch,
                transcript,
                inputs,
                tools: Vec::new(),
            };
            let mut saw_completed = false;
            match model.stream(request).await {
                Ok(mut stream) => {
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(ModelOutputEvent::TextDelta { delta }) => {
                                let _ = tx.send(RuntimeEvent::ModelTextDelta {
                                    event_id: new_id(),
                                    epoch,
                                    delta,
                                });
                            }
                            Ok(ModelOutputEvent::ReasoningDelta { delta }) => {
                                let _ = tx.send(RuntimeEvent::ModelReasoningDelta {
                                    event_id: new_id(),
                                    epoch,
                                    delta,
                                });
                            }
                            Ok(ModelOutputEvent::ToolCall { call }) => {
                                let _ = tx.send(RuntimeEvent::ModelToolCall {
                                    event_id: new_id(),
                                    epoch,
                                    call,
                                });
                            }
                            Ok(ModelOutputEvent::Completed { usage }) => {
                                saw_completed = true;
                                let _ = tx.send(RuntimeEvent::ModelCompleted {
                                    event_id: new_id(),
                                    epoch,
                                    usage,
                                });
                            }
                            Err(err) => {
                                let _ = tx.send(map_agent_error(epoch, err));
                                return;
                            }
                        }
                    }
                    if !saw_completed {
                        let _ = tx.send(RuntimeEvent::FatalError {
                            event_id: new_id(),
                            message: "model stream ended without completed event".to_string(),
                        });
                    }
                }
                Err(err) => {
                    let _ = tx.send(map_agent_error(epoch, err));
                }
            }
        });
    }

    fn spawn_tool_execution(&self, epoch: u64, session_id: String, turn_id: String, call: ToolCall) {
        let tx = self.tx.clone();
        let tools = Arc::clone(&self.tools);
        let semaphore = Arc::clone(&self.semaphore);

        tokio::spawn(async move {
            let _ = tx.send(RuntimeEvent::ToolDispatched {
                event_id: new_id(),
                epoch,
                call_id: call.call_id.clone(),
            });

            let Ok(permit) = semaphore.acquire_owned().await else {
                let _ = tx.send(RuntimeEvent::FatalError {
                    event_id: new_id(),
                    message: "tool semaphore closed".to_string(),
                });
                return;
            };

            let result = tools
                .execute_tool(
                    call.clone(),
                    ToolExecutionContext {
                        session_id,
                        turn_id,
                        epoch,
                        cwd: None,
                    },
                )
                .await;
            drop(permit);

            match result {
                Ok(mut result) => {
                    if result.call_id != call.call_id {
                        result.call_id = call.call_id.clone();
                    }
                    let _ = tx.send(if result.is_error {
                        RuntimeEvent::ToolResultErr {
                            event_id: new_id(),
                            epoch,
                            result,
                        }
                    } else {
                        RuntimeEvent::ToolResultOk {
                            event_id: new_id(),
                            epoch,
                            result,
                        }
                    });
                }
                Err(err) => {
                    let _ = tx.send(RuntimeEvent::ToolResultErr {
                        event_id: new_id(),
                        epoch,
                        result: ToolResult::err(call.call_id, err.message),
                    });
                }
            }
        });
    }

    fn spawn_retry_timer(&self, delay_ms: u64, next_epoch: u64) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            let _ = tx.send(RuntimeEvent::RetryTimerFired {
                event_id: new_id(),
                next_epoch,
            });
        });
    }
}

fn map_agent_error(epoch: u64, err: AgentError) -> RuntimeEvent {
    match err {
        AgentError::Transient(e) => RuntimeEvent::TransientError {
            event_id: new_id(),
            epoch,
            message: e.to_string(),
            retry_after_ms: e.retry_after_ms(),
        },
        other => RuntimeEvent::FatalError {
            event_id: new_id(),
            message: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use async_trait::async_trait;

    struct CountingTool {
        running: Arc<AtomicUsize>,
        max_seen: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ToolExecutor for CountingTool {
        async fn execute_tool(
            &self,
            _call: ToolCall,
            _ctx: ToolExecutionContext,
        ) -> Result<ToolResult, agent_core::tools::ToolExecutionError> {
            let now = self.running.fetch_add(1, Ordering::SeqCst) + 1;
            loop {
                let old = self.max_seen.load(Ordering::SeqCst);
                if now <= old {
                    break;
                }
                if self
                    .max_seen
                    .compare_exchange(old, now, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.running.fetch_sub(1, Ordering::SeqCst);
            Ok(ToolResult::ok(
                _call.call_id,
                serde_json::json!({"ok": true}),
            ))
        }
    }

    struct DummyModel;

    #[async_trait]
    impl agent_core::LanguageModel for DummyModel {
        fn model_name(&self) -> &str {
            "dummy"
        }

        async fn stream(
            &self,
            _request: ModelRequest,
        ) -> Result<agent_core::ModelEventStream, AgentError> {
            let stream = futures::stream::empty();
            Ok(Box::pin(stream))
        }
    }

    #[tokio::test]
    async fn tool_execution_respects_parallel_limit() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let running = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));

        let exec = EffectExecutor::new(
            Arc::new(DummyModel),
            Arc::new(CountingTool {
                running: Arc::clone(&running),
                max_seen: Arc::clone(&max_seen),
            }),
            tx,
            1,
        );

        let call1 = ToolCall::new("tool", serde_json::json!({"n": 1}));
        let call2 = ToolCall::new("tool", serde_json::json!({"n": 2}));

        exec.execute(Effect::ExecuteTool {
            epoch: 0,
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
            call: call1,
        })
        .await;
        exec.execute(Effect::ExecuteTool {
            epoch: 0,
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
            call: call2,
        })
        .await;

        let mut completed = 0;
        while completed < 2 {
            if let Some(ev) = rx.recv().await {
                if matches!(
                    ev,
                    RuntimeEvent::ToolResultOk { .. } | RuntimeEvent::ToolResultErr { .. }
                ) {
                    completed += 1;
                }
            }
        }

        assert_eq!(max_seen.load(Ordering::SeqCst), 1);
    }
}
