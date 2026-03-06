use std::sync::Arc;

use argus_core::{FinishReason, ResponseEvent};
use futures::StreamExt;
use tokio::{
    sync::mpsc,
    task::{self, JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tool::{ToolContext, ToolResult};

use crate::{
    AuthorizationDecision, LlmRequestSnapshot, ModelRunner, PermissionDecision,
    StepFinishReason, ToolAuthorizer, ToolOutcome, ToolRunner, TurnContext, TurnError, TurnEvent,
    TurnFailure, TurnFinishReason, TurnHandle, TurnState, TurnSummary,
    state::{ActiveLlmStep, PendingPermissionCall, PermissionPause, ToolBatch},
};

pub struct TurnDriver {
    context: TurnContext,
    model: Arc<dyn ModelRunner>,
    tool_runner: Arc<dyn ToolRunner>,
    authorizer: Arc<dyn ToolAuthorizer>,
    observer: Arc<dyn crate::TurnObserver>,
    state: TurnState,
    command_rx: mpsc::Receiver<crate::TurnCommand>,
    event_tx: mpsc::Sender<TurnEvent>,
}

impl TurnDriver {
    pub fn spawn(
        context: TurnContext,
        model: Arc<dyn ModelRunner>,
        tool_runner: Arc<dyn ToolRunner>,
        authorizer: Arc<dyn ToolAuthorizer>,
        observer: Arc<dyn crate::TurnObserver>,
    ) -> (TurnHandle, JoinHandle<Result<(), TurnError>>) {
        let (command_tx, command_rx) = mpsc::channel(8);
        let (event_tx, event_rx) = mpsc::channel(32);

        let handle = TurnHandle::new(command_tx, event_rx);
        let driver = Self {
            state: TurnState::Ready(context.clone()),
            context,
            model,
            tool_runner,
            authorizer,
            observer,
            command_rx,
            event_tx,
        };

        let task = task::spawn(async move { driver.run().await });
        (handle, task)
    }

    async fn run(mut self) -> Result<(), TurnError> {
        self.emit(TurnEvent::TurnStarted).await?;
        let mut step_index = 0;

        loop {
            let request = LlmRequestSnapshot {
                session_id: self.context.session_id.clone(),
                turn_id: self.context.turn_id.clone(),
                input_text: self.context.user_message.clone(),
            };

            let mut active_step = ActiveLlmStep {
                step_index,
                tool_calls: Vec::new(),
            };
            self.state = TurnState::StreamingLlm(active_step.clone());
            let mut stream = self.model.start(request).await?;

            let terminal_reason = loop {
                let event = stream.next().await.ok_or_else(|| {
                    TurnError::Runtime("model stream ended before a terminal event".into())
                })?;
                match event {
                    ResponseEvent::ContentDelta(text) => {
                        self.emit(TurnEvent::LlmTextDelta {
                            text: text.to_string(),
                        })
                        .await?;
                    }
                    ResponseEvent::ReasoningDelta(text) => {
                        self.emit(TurnEvent::LlmReasoningDelta {
                            text: text.to_string(),
                        })
                        .await?;
                    }
                    ResponseEvent::ToolDone(call) => {
                        active_step.tool_calls.push(call.clone());
                        self.emit(TurnEvent::ToolCallPrepared { call }).await?;
                    }
                    ResponseEvent::Done { reason, .. } => break reason,
                    ResponseEvent::Error(err) => {
                        self.state = TurnState::Failed(TurnFailure {
                            message: err.message.clone(),
                        });
                        self.emit(TurnEvent::TurnFinished {
                            reason: TurnFinishReason::Failed,
                        })
                        .await?;
                        return Err(TurnError::Runtime(err.message));
                    }
                    ResponseEvent::Created(_)
                    | ResponseEvent::ToolDelta(_)
                    | ResponseEvent::ContentDone(_)
                    | ResponseEvent::ReasoningDone(_) => {}
                }
            };

            match terminal_reason {
                FinishReason::ToolCalls => {
                    let batch = ToolBatch {
                        step_index,
                        calls: active_step.tool_calls,
                    };
                    if batch.calls.is_empty() {
                        return Err(TurnError::Runtime(
                            "finish_reason=tool_calls without any completed tool calls".into(),
                        ));
                    }
                    self.state = TurnState::WaitingTools(batch.clone());
                    self.execute_tool_batch(batch).await?;
                    step_index += 1;
                }
                FinishReason::Cancelled => {
                    let summary = TurnSummary {
                        turn_id: self.context.turn_id.clone(),
                    };
                    self.state = TurnState::Cancelled(summary);
                    self.emit(TurnEvent::TurnFinished {
                        reason: TurnFinishReason::Cancelled,
                    })
                    .await?;
                    return Ok(());
                }
                _ => {
                    let summary = TurnSummary {
                        turn_id: self.context.turn_id.clone(),
                    };
                    self.state = TurnState::Completed(summary);
                    self.emit(TurnEvent::TurnFinished {
                        reason: TurnFinishReason::Completed,
                    })
                    .await?;
                    return Ok(());
                }
            }
        }
    }

    async fn emit(&self, event: TurnEvent) -> Result<(), TurnError> {
        self.observer.on_event(&event).await?;
        self.event_tx
            .send(event)
            .await
            .map_err(|_| TurnError::Runtime("turn event receiver dropped".into()))
    }

    async fn execute_tool_batch(&mut self, batch: ToolBatch) -> Result<(), TurnError> {
        let mut join_set = JoinSet::new();
        let mut pending_permissions = Vec::new();

        for call in batch.calls.clone() {
            match self.authorizer.authorize(&call).await? {
                AuthorizationDecision::Allow => self.spawn_tool_call(&mut join_set, call),
                AuthorizationDecision::Deny => {
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id: call_id(&call),
                        result: ToolOutcome::Denied,
                    })
                    .await?;
                }
                AuthorizationDecision::Ask(request) => {
                    self.emit(TurnEvent::ToolCallPermissionRequested {
                        request: request.clone(),
                    })
                    .await?;
                    pending_permissions.push(PendingPermissionCall { request, call });
                }
            }
        }

        self.state = if pending_permissions.is_empty() {
            TurnState::WaitingTools(batch.clone())
        } else {
            TurnState::WaitingForPermission(PermissionPause {
                batch: batch.clone(),
                pending: pending_permissions.clone(),
            })
        };

        while !join_set.is_empty() || !pending_permissions.is_empty() {
            tokio::select! {
                Some(result) = join_set.join_next(), if !join_set.is_empty() => {
                    let (call_id, result) =
                        result.map_err(|err| TurnError::Runtime(format!("tool task join failed: {err}")))?;
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id,
                        result: map_tool_result(result),
                    })
                    .await?;
                }
                command = self.command_rx.recv(), if !pending_permissions.is_empty() => {
                    let command = command.ok_or_else(|| {
                        TurnError::Runtime("turn command channel closed while waiting for permission".into())
                    })?;
                    if let crate::TurnCommand::ResolvePermission { request_id, decision } = command
                        && let Some(index) = pending_permissions
                            .iter()
                            .position(|pending| pending.request.request_id == request_id)
                    {
                        let pending = pending_permissions.swap_remove(index);
                        self.emit(TurnEvent::ToolCallPermissionResolved {
                            request_id: request_id.clone(),
                            decision: decision.clone(),
                        })
                        .await?;

                        match decision {
                            PermissionDecision::Allow => self.spawn_tool_call(&mut join_set, pending.call),
                            PermissionDecision::Deny => {
                                self.emit(TurnEvent::ToolCallCompleted {
                                    call_id: call_id(&pending.call),
                                    result: ToolOutcome::Denied,
                                })
                                .await?;
                            }
                        }

                        self.state = if pending_permissions.is_empty() {
                            TurnState::WaitingTools(batch.clone())
                        } else {
                            TurnState::WaitingForPermission(PermissionPause {
                                batch: batch.clone(),
                                pending: pending_permissions.clone(),
                            })
                        };
                    }
                }
            }
        }

        self.emit(TurnEvent::StepFinished {
            step_index: batch.step_index,
            reason: StepFinishReason::ToolCalls,
        })
        .await?;
        Ok(())
    }

    fn spawn_tool_call(
        &self,
        join_set: &mut JoinSet<(String, Result<ToolResult, TurnError>)>,
        call: argus_core::ToolCall,
    ) {
        let tool_runner = Arc::clone(&self.tool_runner);
        let session_id = self.context.session_id.clone();
        let turn_id = self.context.turn_id.clone();
        join_set.spawn(async move {
            let call_id = call_id(&call);
            let result = tool_runner
                .execute(
                    call,
                    ToolContext::new(session_id, turn_id, CancellationToken::new()),
                )
                .await;
            (call_id, result)
        });
    }
}

fn call_id(call: &argus_core::ToolCall) -> String {
    match call {
        argus_core::ToolCall::FunctionCall { call_id, .. } => call_id.clone(),
        argus_core::ToolCall::Builtin(call) => call.call_id.clone(),
        argus_core::ToolCall::Mcp(call) => call.id.clone(),
    }
}

fn map_tool_result(result: Result<ToolResult, TurnError>) -> ToolOutcome {
    match result {
        Ok(result) if result.is_error => ToolOutcome::Failed {
            message: result.output.to_string(),
            retryable: false,
        },
        Ok(result) => ToolOutcome::Success(result.output),
        Err(err) => ToolOutcome::Failed {
            message: err.to_string(),
            retryable: false,
        },
    }
}
