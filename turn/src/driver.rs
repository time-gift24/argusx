use std::{collections::HashMap, sync::Arc, time::Instant};

use argus_core::{FinishReason, ResponseEvent};
use futures::StreamExt;
use tokio::{
    sync::mpsc,
    task::{self, JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tool::{ToolContext, ToolResult};

use crate::{
    AuthorizationDecision, FinalStepPolicy, LlmStepRequest, ModelRunner, PermissionDecision,
    StepFinishReason, ToolAuthorizer, ToolOutcome, ToolRunner, TurnContext, TurnError, TurnEvent,
    TurnFailure, TurnFinishReason, TurnHandle, TurnMessage, TurnOptions, TurnState, TurnSummary,
    TurnTranscript,
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
    transcript: TurnTranscript,
    turn_start: Instant,
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
            transcript: TurnTranscript::new(),
            turn_start: Instant::now(),
        };

        let task = task::spawn(async move { driver.run().await });
        (handle, task)
    }

    async fn run(mut self) -> Result<(), TurnError> {
        self.emit(TurnEvent::TurnStarted).await?;
        self.transcript.push(TurnMessage::User {
            content: self.context.user_message.clone(),
        });

        let mut step_index: u32 = 0;

        loop {
            // --- turn deadline ---
            if self.turn_start.elapsed() >= self.options.turn_deadline {
                return self.finish(TurnFinishReason::LlmTimeout).await;
            }

            // --- cancellation ---
            if self.consume_cancel_request()? {
                return self.finish_cancelled().await;
            }

            // --- step limit ---
            let allow_tools = match self.options.final_step_policy {
                FinalStepPolicy::Fail => {
                    if step_index >= self.options.max_steps {
                        return self.finish(TurnFinishReason::MaxStepsExceeded).await;
                    }
                    true
                }
                FinalStepPolicy::ForceText => {
                    if step_index > self.options.max_steps {
                        // Hard ceiling: forced-text step returned tool calls — protocol error.
                        return self.finish(TurnFinishReason::MaxStepsExceeded).await;
                    }
                    step_index < self.options.max_steps
                }
            };

            // --- build request from transcript ---
            let request = LlmStepRequest {
                session_id: self.context.session_id.clone(),
                turn_id: self.context.turn_id.clone(),
                step_index,
                messages: self.transcript.messages().to_vec(),
                allow_tools,
            };

            // --- start model (with model_start_timeout) ---
            let mut active_step = ActiveLlmStep {
                step_index,
                tool_calls: Vec::new(),
            };
            self.state = TurnState::StreamingLlm(active_step.clone());
            let model = Arc::clone(&self.model);
            let start_fut = async move { model.start(request).await };
            tokio::pin!(start_fut);

            let mut stream = loop {
                tokio::select! {
                    biased;
                    command = self.command_rx.recv() => {
                        if matches!(command, Some(crate::TurnCommand::Cancel)) {
                            self.cancel_token.cancel();
                            return self.finish_cancelled().await;
                        }
                    }
                    result = tokio::time::timeout(
                        self.options.model_start_timeout,
                        &mut start_fut,
                    ) => {
                        match result {
                            Ok(Ok(stream)) => break stream,
                            Ok(Err(e)) => return Err(e),
                            Err(_elapsed) => {
                                return self.finish(TurnFinishReason::LlmTimeout).await;
                            }
                        }
                    }
                }
            };

            // --- stream (with stream_idle_timeout per event) ---
            let terminal_reason = loop {
                tokio::select! {
                    biased;
                    command = self.command_rx.recv() => {
                        if matches!(command, Some(crate::TurnCommand::Cancel)) {
                            self.cancel_token.cancel();
                            return self.finish_cancelled().await;
                        }
                    }
                    result = tokio::time::timeout(
                        self.options.stream_idle_timeout,
                        stream.next(),
                    ) => {
                        match result {
                            Err(_elapsed) => {
                                return self.finish(TurnFinishReason::LlmTimeout).await;
                            }
                            Ok(None) => {
                                return Err(TurnError::Runtime(
                                    "model stream ended before a terminal event".into(),
                                ));
                            }
                            Ok(Some(event)) => match event {
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
                            },
                        }
                    }
                }
            };

            // --- dispatch on terminal reason ---
            match terminal_reason {
                FinishReason::Stop => {
                    // Text deltas were emitted in real-time above but are not
                    // buffered here. Buffering them into the transcript is a
                    // follow-up item; the turn is semantically complete on Stop.
                    let summary = TurnSummary {
                        turn_id: self.context.turn_id.clone(),
                    };
                    self.state = TurnState::Completed(summary);
                    return self.finish(TurnFinishReason::Completed).await;
                }
                FinishReason::ToolCalls => {
                    if active_step.tool_calls.is_empty() {
                        return Err(TurnError::Runtime(
                            "finish_reason=tool_calls without any completed tool calls".into(),
                        ));
                    }
                    if self.consume_cancel_request()? {
                        return self.finish_cancelled().await;
                    }

                    // Append assistant tool-call message to transcript.
                    self.transcript.push(TurnMessage::AssistantToolCalls {
                        content: None,
                        calls: active_step.tool_calls.clone(),
                    });

                    let batch = ToolBatch {
                        step_index,
                        calls: active_step.tool_calls.clone(),
                    };
                    self.state = TurnState::WaitingTools(batch.clone());

                    let outcomes = self.execute_tool_batch(batch).await?;
                    if matches!(self.state, TurnState::Cancelled(_)) {
                        return Ok(());
                    }

                    // Append tool results in original call order.
                    for (call, outcome) in active_step.tool_calls.iter().zip(outcomes.iter()) {
                        self.transcript.push(TurnMessage::ToolResult {
                            call_id: call_id_str(call),
                            tool_name: tool_name_str(call),
                            content: outcome_to_content(outcome),
                            is_error: outcome_is_error(outcome),
                        });
                    }

                    step_index += 1;
                }
                FinishReason::Length => {
                    return self.finish(TurnFinishReason::ModelLengthLimit).await;
                }
                FinishReason::Cancelled => {
                    return self.finish_cancelled().await;
                }
                FinishReason::Unknown(_) => {
                    return self.finish(TurnFinishReason::ModelProtocolError).await;
                }
            }
        }
    }

    async fn finish(&mut self, reason: TurnFinishReason) -> Result<(), TurnError> {
        self.emit(TurnEvent::TurnFinished { reason }).await
    }

    async fn emit(&self, event: TurnEvent) -> Result<(), TurnError> {
        self.observer.on_event(&event).await?;
        self.event_tx
            .send(event)
            .await
            .map_err(|_| TurnError::Runtime("turn event receiver dropped".into()))
    }

    /// Execute all tool calls in the batch concurrently.
    ///
    /// Returns outcomes in the **same order as `batch.calls`**.
    async fn execute_tool_batch(&mut self, batch: ToolBatch) -> Result<Vec<ToolOutcome>, TurnError> {
        let mut join_set = JoinSet::new();
        let mut pending_permissions = Vec::new();
        let mut outcome_map: HashMap<String, ToolOutcome> = HashMap::new();
        let mut cancellation_requested = false;

        if self.consume_cancel_request()? {
            self.finish_cancelled().await?;
            return Ok(vec![]);
        }

        for call in &batch.calls {
            if self.consume_cancel_request()? {
                self.finish_cancelled().await?;
                return Ok(vec![]);
            }

            match self.authorizer.authorize(call).await? {
                AuthorizationDecision::Allow => {
                    self.spawn_tool_call(&mut join_set, call.clone());
                }
                AuthorizationDecision::Deny => {
                    let cid = call_id_str(call);
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id: cid.clone(),
                        result: ToolOutcome::Denied,
                    })
                    .await?;
                    outcome_map.insert(cid, ToolOutcome::Denied);
                }
                AuthorizationDecision::Ask(request) => {
                    self.emit(TurnEvent::ToolCallPermissionRequested {
                        request: request.clone(),
                    })
                    .await?;
                    pending_permissions.push(PendingPermissionCall {
                        request,
                        call: call.clone(),
                    });
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
                        TurnError::Runtime(
                            "turn command channel closed while waiting for tool results".into(),
                        )
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
                                .position(|p| p.request.request_id == request_id)
                            {
                                let pending = pending_permissions.swap_remove(index);
                                self.emit(TurnEvent::ToolCallPermissionResolved {
                                    request_id: request_id.clone(),
                                    decision: decision.clone(),
                                })
                                .await?;

                                match decision {
                                    PermissionDecision::Allow => {
                                        self.spawn_tool_call(&mut join_set, pending.call);
                                    }
                                    PermissionDecision::Deny => {
                                        let cid = call_id_str(&pending.call);
                                        self.emit(TurnEvent::ToolCallCompleted {
                                            call_id: cid.clone(),
                                            result: ToolOutcome::Denied,
                                        })
                                        .await?;
                                        outcome_map.insert(cid, ToolOutcome::Denied);
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
                    let (cid, task_result) = result
                        .map_err(|e| TurnError::Runtime(format!("tool task join failed: {e}")))?;
                    let outcome = map_tool_result(task_result);
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id: cid.clone(),
                        result: outcome.clone(),
                    })
                    .await?;
                    outcome_map.insert(cid, outcome);
                }
            }
        }

        if cancellation_requested {
            self.finish_cancelled().await?;
            return Ok(vec![]);
        }

        self.emit(TurnEvent::StepFinished {
            step_index: batch.step_index,
            reason: StepFinishReason::ToolCalls,
        })
        .await?;

        // Return outcomes in original call order.
        let ordered = batch
            .calls
            .iter()
            .map(|call| {
                outcome_map
                    .remove(&call_id_str(call))
                    .unwrap_or(ToolOutcome::Denied)
            })
            .collect();

        Ok(ordered)
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
            let cid = call_id_str(&call);
            let result = match tokio::time::timeout(
                timeout,
                tool_runner.execute(call, ToolContext::new(session_id, turn_id, cancel_token)),
            )
            .await
            {
                Ok(result) => ToolTaskResult::Completed(result),
                Err(_) => ToolTaskResult::TimedOut,
            };
            (cid, result)
        });
    }

    async fn finish_cancelled(&mut self) -> Result<(), TurnError> {
        self.state = TurnState::Cancelled(TurnSummary {
            turn_id: self.context.turn_id.clone(),
        });
        self.emit(TurnEvent::TurnFinished {
            reason: TurnFinishReason::Cancelled,
        })
        .await
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

// ---------------------------------------------------------------------------
// Pure utility functions (no self)
// ---------------------------------------------------------------------------

fn call_id_str(call: &argus_core::ToolCall) -> String {
    match call {
        argus_core::ToolCall::FunctionCall { call_id, .. } => call_id.clone(),
        argus_core::ToolCall::Builtin(c) => c.call_id.clone(),
        argus_core::ToolCall::Mcp(c) => c.id.clone(),
    }
}

fn tool_name_str(call: &argus_core::ToolCall) -> String {
    match call {
        argus_core::ToolCall::FunctionCall { name, .. } => name.clone(),
        argus_core::ToolCall::Builtin(c) => c.builtin.canonical_name().to_string(),
        argus_core::ToolCall::Mcp(c) => c.name.clone().unwrap_or_default(),
    }
}

fn outcome_to_content(outcome: &ToolOutcome) -> String {
    match outcome {
        ToolOutcome::Success(v) => v.to_string(),
        ToolOutcome::Failed { message, .. } => message.clone(),
        ToolOutcome::TimedOut => "tool timed out".into(),
        ToolOutcome::Denied => "tool call denied".into(),
        ToolOutcome::Cancelled => "tool call cancelled".into(),
    }
}

fn outcome_is_error(outcome: &ToolOutcome) -> bool {
    !matches!(outcome, ToolOutcome::Success(_))
}

fn map_tool_result(result: ToolTaskResult) -> ToolOutcome {
    match result {
        ToolTaskResult::Completed(Ok(r)) if r.is_error => ToolOutcome::Failed {
            message: r.output.to_string(),
            retryable: false,
        },
        ToolTaskResult::Completed(Ok(r)) => ToolOutcome::Success(r.output),
        ToolTaskResult::Completed(Err(e)) => ToolOutcome::Failed {
            message: e.to_string(),
            retryable: false,
        },
        ToolTaskResult::TimedOut => ToolOutcome::TimedOut,
    }
}
