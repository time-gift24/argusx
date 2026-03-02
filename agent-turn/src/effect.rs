use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use agent_core::{
    new_id,
    tools::{ToolCatalog, ToolExecutionContext, ToolExecutor, ToolParallelMode},
    AgentError, LanguageModel, ModelOutputEvent, ModelRequest, RuntimeEvent, ToolCall, ToolResult,
    TranscriptItem,
};
use futures::StreamExt;
use tokio::sync::{mpsc, Mutex as AsyncMutex, Semaphore};

#[derive(Debug, Clone)]
pub enum Effect {
    StartModel {
        epoch: u64,
        provider: String,
        model: String,
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
    T: ToolExecutor + ToolCatalog + 'static,
{
    model: Arc<L>,
    tools: Arc<T>,
    tx: mpsc::UnboundedSender<RuntimeEvent>,
    semaphore: Arc<Semaphore>,
    exclusive_tool_locks: Arc<StdMutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
}

impl<L, T> EffectExecutor<L, T>
where
    L: LanguageModel + 'static,
    T: ToolExecutor + ToolCatalog + 'static,
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
            exclusive_tool_locks: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    pub async fn execute(&self, effect: Effect) {
        match effect {
            Effect::StartModel {
                epoch,
                provider,
                model,
                transcript,
                inputs,
            } => {
                self.spawn_model_stream(epoch, provider, model, transcript, inputs);
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
        provider: String,
        model: String,
        transcript: Vec<TranscriptItem>,
        inputs: Vec<agent_core::InputEnvelope>,
    ) {
        let model_adapter = Arc::clone(&self.model);
        let tools = Arc::clone(&self.tools);
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let tool_specs = tools.list_tools().await;
            let request = ModelRequest {
                epoch,
                provider,
                model,
                transcript,
                inputs,
                tools: tool_specs,
            };
            let mut saw_completed = false;
            match model_adapter.stream(request).await {
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

    fn spawn_tool_execution(
        &self,
        epoch: u64,
        session_id: String,
        turn_id: String,
        call: ToolCall,
    ) {
        let tx = self.tx.clone();
        let tools = Arc::clone(&self.tools);
        let semaphore = Arc::clone(&self.semaphore);
        let exclusive_tool_locks = Arc::clone(&self.exclusive_tool_locks);

        tokio::spawn(async move {
            let _ = tx.send(RuntimeEvent::ToolQueued {
                event_id: new_id(),
                epoch,
                call_id: call.call_id.clone(),
                tool_name: call.tool_name.clone(),
            });

            let Ok(permit) = semaphore.acquire_owned().await else {
                let _ = tx.send(RuntimeEvent::FatalError {
                    event_id: new_id(),
                    message: "tool semaphore closed".to_string(),
                });
                return;
            };

            let _ = tx.send(RuntimeEvent::ToolDequeued {
                event_id: new_id(),
                epoch,
                call_id: call.call_id.clone(),
                tool_name: call.tool_name.clone(),
            });
            let _ = tx.send(RuntimeEvent::ToolDispatched {
                event_id: new_id(),
                epoch,
                call_id: call.call_id.clone(),
            });

            let ctx = ToolExecutionContext {
                session_id,
                turn_id,
                epoch,
                cwd: None,
            };
            let mode = tools
                .tool_spec(&call.tool_name)
                .await
                .map(|spec| spec.execution_policy.parallel_mode)
                .unwrap_or(ToolParallelMode::ParallelSafe);
            let started_at = Instant::now();

            let result = if matches!(mode, ToolParallelMode::Exclusive) {
                let lock = {
                    let mut locks = exclusive_tool_locks
                        .lock()
                        .expect("exclusive tool lock map poisoned");
                    locks
                        .entry(call.tool_name.clone())
                        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                        .clone()
                };
                let _exclusive_guard = lock.lock().await;
                tools.execute_tool(call.clone(), ctx).await
            } else {
                tools.execute_tool(call.clone(), ctx).await
            };
            drop(permit);

            match result {
                Ok(mut result) => {
                    let duration_ms = started_at.elapsed().as_millis() as u64;
                    if let Some(output) = result.output.as_object_mut() {
                        output.insert("duration_ms".to_string(), serde_json::json!(duration_ms));
                    }
                    emit_terminal_runtime_events(&tx, epoch, &call.call_id, &result, duration_ms);
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
                    let duration_ms = started_at.elapsed().as_millis() as u64;
                    let message = err.message;
                    let _ = tx.send(RuntimeEvent::ToolStderrDelta {
                        event_id: new_id(),
                        epoch,
                        call_id: call.call_id.clone(),
                        delta: message.clone(),
                    });
                    let _ = tx.send(RuntimeEvent::ToolExit {
                        event_id: new_id(),
                        epoch,
                        call_id: call.call_id.clone(),
                        exit_code: None,
                        duration_ms,
                    });
                    let _ = tx.send(RuntimeEvent::ToolResultErr {
                        event_id: new_id(),
                        epoch,
                        result: ToolResult {
                            call_id: call.call_id,
                            output: serde_json::json!({
                                "error": message,
                                "duration_ms": duration_ms,
                            }),
                            is_error: true,
                        },
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

fn emit_terminal_runtime_events(
    tx: &mpsc::UnboundedSender<RuntimeEvent>,
    epoch: u64,
    call_id: &str,
    result: &ToolResult,
    duration_ms: u64,
) {
    let stdout = result
        .output
        .get("stdout")
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty());
    if let Some(stdout) = stdout {
        let _ = tx.send(RuntimeEvent::ToolStdoutDelta {
            event_id: new_id(),
            epoch,
            call_id: call_id.to_string(),
            delta: stdout.to_string(),
        });
    }

    let stderr = result
        .output
        .get("stderr")
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty());
    if let Some(stderr) = stderr {
        let _ = tx.send(RuntimeEvent::ToolStderrDelta {
            event_id: new_id(),
            epoch,
            call_id: call_id.to_string(),
            delta: stderr.to_string(),
        });
    }

    let exit_code = result
        .output
        .get("exit_code")
        .and_then(serde_json::Value::as_i64)
        .and_then(|v| i32::try_from(v).ok());
    let _ = tx.send(RuntimeEvent::ToolExit {
        event_id: new_id(),
        epoch,
        call_id: call_id.to_string(),
        exit_code,
        duration_ms,
    });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use agent_core::tools::{ToolExecutionPolicy, ToolParallelMode, ToolSpec};
    use async_trait::async_trait;

    struct CountingTool {
        running: Arc<AtomicUsize>,
        max_seen: Arc<AtomicUsize>,
        mode: ToolParallelMode,
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

    #[async_trait]
    impl ToolCatalog for CountingTool {
        async fn list_tools(&self) -> Vec<ToolSpec> {
            vec![self.spec_for("tool")]
        }

        async fn tool_spec(&self, name: &str) -> Option<ToolSpec> {
            (name == "tool").then(|| self.spec_for(name))
        }
    }

    impl CountingTool {
        fn spec_for(&self, name: &str) -> ToolSpec {
            ToolSpec {
                name: name.to_string(),
                description: "test tool".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
                execution_policy: ToolExecutionPolicy {
                    parallel_mode: self.mode.clone(),
                    timeout_ms: None,
                    retry: None,
                },
            }
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
                mode: ToolParallelMode::ParallelSafe,
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

    #[tokio::test]
    async fn exclusive_tools_are_serialized_even_with_parallel_slots() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let running = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));

        let exec = EffectExecutor::new(
            Arc::new(DummyModel),
            Arc::new(CountingTool {
                running: Arc::clone(&running),
                max_seen: Arc::clone(&max_seen),
                mode: ToolParallelMode::Exclusive,
            }),
            tx,
            4,
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
