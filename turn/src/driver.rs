use std::sync::Arc;

use argus_core::{FinishReason, ResponseEvent};
use futures::StreamExt;
use tokio::{
    sync::mpsc,
    task::{self, JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tool::{ToolContext, ToolResult};
use tracing::{Instrument, info, info_span};

use crate::{
    AuthorizationDecision, LlmRequestSnapshot, ModelRunner, PermissionDecision, StepFinishReason,
    ToolAuthorizer, ToolOutcome, ToolRunner, TurnContext, TurnError, TurnEvent, TurnFailure,
    TurnFinishReason, TurnHandle, TurnOptions, TurnState, TurnSummary,
    state::{ActiveLlmStep, PendingPermissionCall, PermissionPause, ToolBatch},
};

enum ToolTaskResult {
    Completed(Result<ToolResult, TurnError>),
    TimedOut,
}

pub struct TurnDriver {
    context: TurnContext,
    model: Arc<dyn ModelRunner>,
    tool_runner: Arc<dyn ToolRunner>,
    authorizer: Arc<dyn ToolAuthorizer>,
    observer: Arc<dyn crate::TurnObserver>,
    state: TurnState,
    command_rx: mpsc::Receiver<crate::TurnCommand>,
    event_tx: mpsc::Sender<TurnEvent>,
    cancel_token: CancellationToken,
    options: TurnOptions,
}

impl TurnDriver {
    pub fn spawn(
        context: TurnContext,
        model: Arc<dyn ModelRunner>,
        tool_runner: Arc<dyn ToolRunner>,
        authorizer: Arc<dyn ToolAuthorizer>,
        observer: Arc<dyn crate::TurnObserver>,
    ) -> (TurnHandle, JoinHandle<Result<(), TurnError>>) {
        Self::spawn_with_options(
            context,
            TurnOptions::default(),
            model,
            tool_runner,
            authorizer,
            observer,
        )
    }

    pub fn spawn_with_options(
        context: TurnContext,
        options: TurnOptions,
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
            cancel_token: CancellationToken::new(),
            options,
        };
        let span = info_span!(
            "turn.run",
            session_id = %driver.context.session_id,
            turn_id = %driver.context.turn_id
        );

        let task = task::spawn(async move { driver.run().await }.instrument(span));
        (handle, task)
    }

    async fn run(mut self) -> Result<(), TurnError> {
        info!("turn started");
        self.emit(TurnEvent::TurnStarted).await?;
        let mut step_index = 0;

        loop {
            if self.consume_cancel_request()? {
                self.finish_cancelled().await?;
                return Ok(());
            }

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
            let model = Arc::clone(&self.model);
            let start = async move { model.start(request).await };
            tokio::pin!(start);
            let mut stream = loop {
                tokio::select! {
                    biased;
                    command = self.command_rx.recv() => {
                        if matches!(command, Some(crate::TurnCommand::Cancel)) {
                            self.cancel_token.cancel();
                            self.finish_cancelled().await?;
                            return Ok(());
                        }
                    }
                    result = &mut start => break result?,
                }
            };

            let terminal_reason = loop {
                tokio::select! {
                    biased;
                    command = self.command_rx.recv() => {
                        if matches!(command, Some(crate::TurnCommand::Cancel)) {
                            self.cancel_token.cancel();
                            self.finish_cancelled().await?;
                            return Ok(());
                        }
                    }
                    event = stream.next() => {
                        let event = event.ok_or_else(|| {
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
                                self.emit_turn_finished(TurnFinishReason::Failed).await?;
                                return Err(TurnError::Runtime(err.message));
                            }
                            ResponseEvent::Created(_)
                            | ResponseEvent::ToolDelta(_)
                            | ResponseEvent::ContentDone(_)
                            | ResponseEvent::ReasoningDone(_) => {}
                        }
                    }
                }
            };
            info!(step_index, finish_reason = ?terminal_reason, "step finished");

            match terminal_reason {
                FinishReason::ToolCalls => {
                    if self.consume_cancel_request()? {
                        self.finish_cancelled().await?;
                        return Ok(());
                    }
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
                    if matches!(self.state, TurnState::Cancelled(_)) {
                        return Ok(());
                    }
                    step_index += 1;
                }
                FinishReason::Cancelled => {
                    let summary = TurnSummary {
                        turn_id: self.context.turn_id.clone(),
                    };
                    self.state = TurnState::Cancelled(summary);
                    self.emit_turn_finished(TurnFinishReason::Cancelled).await?;
                    return Ok(());
                }
                _ => {
                    let summary = TurnSummary {
                        turn_id: self.context.turn_id.clone(),
                    };
                    self.state = TurnState::Completed(summary);
                    self.emit_turn_finished(TurnFinishReason::Completed).await?;
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

    async fn emit_turn_finished(&self, reason: TurnFinishReason) -> Result<(), TurnError> {
        info!(reason = ?reason, "turn finished");
        self.emit(TurnEvent::TurnFinished { reason }).await
    }

    async fn execute_tool_batch(&mut self, batch: ToolBatch) -> Result<(), TurnError> {
        info!(
            step_index = batch.step_index,
            tool_count = batch.calls.len(),
            "tool batch prepared"
        );
        let mut join_set = JoinSet::new();
        let mut pending_permissions = Vec::new();
        let mut cancellation_requested = false;

        if self.consume_cancel_request()? {
            self.finish_cancelled().await?;
            return Ok(());
        }

        for call in batch.calls.clone() {
            if self.consume_cancel_request()? {
                self.finish_cancelled().await?;
                return Ok(());
            }

            match self.authorizer.authorize(&call).await? {
                AuthorizationDecision::Allow => self.spawn_tool_call(&mut join_set, call),
                AuthorizationDecision::Deny => {
                    info!(call_id = %call_id(&call), "tool call denied");
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id: call_id(&call),
                        result: ToolOutcome::Denied,
                    })
                    .await?;
                }
                AuthorizationDecision::Ask(request) => {
                    info!(request_id = %request.request_id, tool_call_id = %request.tool_call_id, "tool permission requested");
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
                biased;
                command = self.command_rx.recv() => {
                    let command = command.ok_or_else(|| {
                        TurnError::Runtime("turn command channel closed while waiting for tool results".into())
                    })?;
                    match command {
                        crate::TurnCommand::Cancel => {
                            self.cancel_token.cancel();
                            pending_permissions.clear();
                            cancellation_requested = true;
                            self.state = TurnState::Cancelled(TurnSummary {
                                turn_id: self.context.turn_id.clone(),
                            });
                        }
                        crate::TurnCommand::ResolvePermission { request_id, decision } => {
                            if let Some(index) = pending_permissions
                                .iter()
                                .position(|pending| pending.request.request_id == request_id)
                            {
                                let pending = pending_permissions.swap_remove(index);
                                info!(request_id = %request_id, tool_call_id = %pending.request.tool_call_id, decision = ?decision, "tool permission resolved");
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
                Some(result) = join_set.join_next(), if !join_set.is_empty() => {
                    let (call_id, result) =
                        result.map_err(|err| TurnError::Runtime(format!("tool task join failed: {err}")))?;
                    info!(call_id = %call_id, "tool call completed");
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id,
                        result: map_tool_result(result),
                    })
                    .await?;
                }
            }
        }

        if cancellation_requested {
            self.finish_cancelled().await?;
            return Ok(());
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
        join_set: &mut JoinSet<(String, ToolTaskResult)>,
        call: argus_core::ToolCall,
    ) {
        let tool_runner = Arc::clone(&self.tool_runner);
        let session_id = self.context.session_id.clone();
        let turn_id = self.context.turn_id.clone();
        let cancel_token = self.cancel_token.child_token();
        let timeout = self.options.tool_timeout;
        join_set.spawn(async move {
            let call_id = call_id(&call);
            let result = match tokio::time::timeout(
                timeout,
                tool_runner.execute(call, ToolContext::new(session_id, turn_id, cancel_token)),
            )
            .await
            {
                Ok(result) => ToolTaskResult::Completed(result),
                Err(_) => ToolTaskResult::TimedOut,
            };
            (call_id, result)
        });
    }

    async fn finish_cancelled(&mut self) -> Result<(), TurnError> {
        self.state = TurnState::Cancelled(TurnSummary {
            turn_id: self.context.turn_id.clone(),
        });
        self.emit_turn_finished(TurnFinishReason::Cancelled).await
    }

    fn consume_cancel_request(&mut self) -> Result<bool, TurnError> {
        loop {
            match self.command_rx.try_recv() {
                Ok(crate::TurnCommand::Cancel) => {
                    self.cancel_token.cancel();
                    return Ok(true);
                }
                Ok(crate::TurnCommand::ResolvePermission { .. }) => {}
                Err(mpsc::error::TryRecvError::Empty) => return Ok(false),
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(TurnError::Runtime("turn command receiver dropped".into()));
                }
            }
        }
    }
}

fn call_id(call: &argus_core::ToolCall) -> String {
    match call {
        argus_core::ToolCall::FunctionCall { call_id, .. } => call_id.clone(),
        argus_core::ToolCall::Builtin(call) => call.call_id.clone(),
        argus_core::ToolCall::Mcp(call) => call.id.clone(),
    }
}

fn map_tool_result(result: ToolTaskResult) -> ToolOutcome {
    match result {
        ToolTaskResult::Completed(Ok(result)) if result.is_error => ToolOutcome::Failed {
            message: result.output.to_string(),
            retryable: false,
        },
        ToolTaskResult::Completed(Ok(result)) => ToolOutcome::Success(result.output),
        ToolTaskResult::Completed(Err(err)) => ToolOutcome::Failed {
            message: err.to_string(),
            retryable: false,
        },
        ToolTaskResult::TimedOut => ToolOutcome::TimedOut,
    }
}
