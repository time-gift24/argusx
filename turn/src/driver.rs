use std::{sync::Arc, time::Instant};

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
    AuthorizationDecision, FinalStepPolicy, LlmStepRequest, ModelRunner, PermissionDecision,
    StepFinishReason, ToolAuthorizer, ToolOutcome, ToolRunner, TurnError, TurnEvent, TurnFailure,
    TurnFinishReason, TurnHandle, TurnMessage, TurnOptions, TurnOutcome, TurnSeed, TurnState,
    TurnSummary, TurnTranscript,
    state::{ActiveLlmStep, PendingPermissionCall, PermissionPause, ToolBatch},
    transcript::SharedToolCall,
};

enum ToolTaskResult {
    Completed(Result<ToolResult, TurnError>),
    TimedOut,
}

pub struct TurnDriver {
    seed: TurnSeed,
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
        seed: TurnSeed,
        model: Arc<dyn ModelRunner>,
        tool_runner: Arc<dyn ToolRunner>,
        authorizer: Arc<dyn ToolAuthorizer>,
        observer: Arc<dyn crate::TurnObserver>,
    ) -> (TurnHandle, JoinHandle<Result<TurnOutcome, TurnError>>) {
        Self::spawn_with_options(
            seed,
            TurnOptions::default(),
            model,
            tool_runner,
            authorizer,
            observer,
        )
    }

    pub fn spawn_with_options(
        seed: TurnSeed,
        options: TurnOptions,
        model: Arc<dyn ModelRunner>,
        tool_runner: Arc<dyn ToolRunner>,
        authorizer: Arc<dyn ToolAuthorizer>,
        observer: Arc<dyn crate::TurnObserver>,
    ) -> (TurnHandle, JoinHandle<Result<TurnOutcome, TurnError>>) {
        let (command_tx, command_rx) = mpsc::channel(8);
        let (event_tx, event_rx) = mpsc::channel(32);

        let handle = TurnHandle::new(command_tx, event_rx);
        let driver = Self {
            state: TurnState::Ready,
            seed,
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
        let span = info_span!(
            "turn.run",
            session_id = %driver.seed.session_id,
            turn_id = %driver.seed.turn_id
        );

        let task = task::spawn(async move { driver.run().await }.instrument(span));
        (handle, task)
    }

    async fn run(mut self) -> Result<TurnOutcome, TurnError> {
        info!("turn started");
        self.emit(TurnEvent::TurnStarted).await?;
        for message in &self.seed.prior_messages {
            self.transcript.push(message.clone());
        }
        self.transcript.push(TurnMessage::User {
            content: self.seed.user_message.as_str().into(),
        });

        let mut step_index: u32 = 0;

        loop {
            if self.turn_start.elapsed() >= self.options.turn_deadline {
                return self.finish(TurnFinishReason::LlmTimeout).await;
            }

            if self.consume_cancel_request()? {
                return self.finish_cancelled().await;
            }

            let allow_tools = match self.options.final_step_policy {
                FinalStepPolicy::Fail => {
                    if step_index >= self.options.max_steps {
                        return self.finish(TurnFinishReason::MaxStepsExceeded).await;
                    }
                    true
                }
                FinalStepPolicy::ForceText => {
                    if step_index > self.options.max_steps {
                        return self.finish(TurnFinishReason::MaxStepsExceeded).await;
                    }
                    step_index < self.options.max_steps
                }
            };

            let request = LlmStepRequest {
                session_id: self.seed.session_id.clone(),
                turn_id: self.seed.turn_id.clone(),
                step_index,
                messages: self.transcript.snapshot(),
                allow_tools,
            };

            self.state = TurnState::StreamingLlm(ActiveLlmStep {
                step_index,
                tool_calls: Vec::new(),
            });
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
                            Ok(Err(err)) => return Err(err),
                            Err(_elapsed) => {
                                return self.finish(TurnFinishReason::LlmTimeout).await;
                            }
                        }
                    }
                }
            };

            let mut assistant_text = String::new();
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
                                    assistant_text.push_str(text.as_ref());
                                    self.emit(TurnEvent::LlmTextDelta { text }).await?;
                                }
                                ResponseEvent::ReasoningDelta(text) => {
                                    self.emit(TurnEvent::LlmReasoningDelta { text }).await?;
                                }
                                ResponseEvent::ToolDone(call) => {
                                    let call = Arc::new(call);
                                    {
                                        let active_step = self.streaming_step_mut()?;
                                        active_step.tool_calls.push(Arc::clone(&call));
                                    }
                                    self.emit(TurnEvent::ToolCallPrepared { call }).await?;
                                }
                                ResponseEvent::Done { reason, .. } => break reason,
                                ResponseEvent::Error(err) => {
                                    self.state = TurnState::Failed(TurnFailure {
                                        message: err.message.clone(),
                                    });
                                    self.finish(TurnFinishReason::Failed).await?;
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
            info!(step_index, finish_reason = ?terminal_reason, "step finished");

            match terminal_reason {
                FinishReason::Stop => {
                    if !assistant_text.is_empty() {
                        self.transcript.push(TurnMessage::AssistantText {
                            content: assistant_text.clone().into(),
                        });
                    }
                    let summary = TurnSummary {
                        turn_id: self.seed.turn_id.clone(),
                    };
                    self.state = TurnState::Completed(summary);
                    return self.finish(TurnFinishReason::Completed).await;
                }
                FinishReason::ToolCalls => {
                    let active_step = self.take_streaming_step()?;
                    if active_step.tool_calls.is_empty() {
                        return Err(TurnError::Runtime(
                            "finish_reason=tool_calls without any completed tool calls".into(),
                        ));
                    }
                    if self.consume_cancel_request()? {
                        return self.finish_cancelled().await;
                    }

                    let calls = Arc::from(active_step.tool_calls);
                    self.transcript.push(TurnMessage::AssistantToolCalls {
                        content: (!assistant_text.is_empty())
                            .then(|| assistant_text.clone().into()),
                        calls: Arc::clone(&calls),
                    });

                    let batch = ToolBatch {
                        step_index,
                        calls: Arc::clone(&calls),
                    };
                    self.state = TurnState::WaitingTools(batch.clone());

                    let outcomes = self.execute_tool_batch(batch).await?;
                    if matches!(self.state, TurnState::Cancelled(_)) {
                        return Ok(self.build_outcome(TurnFinishReason::Cancelled));
                    }

                    for (call, outcome) in calls.iter().zip(outcomes.iter()) {
                        self.transcript.push(TurnMessage::ToolResult {
                            call_id: call_id_arc(call.as_ref()),
                            tool_name: tool_name_arc(call.as_ref()),
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

    async fn finish(&mut self, reason: TurnFinishReason) -> Result<TurnOutcome, TurnError> {
        info!(reason = ?reason, "turn finished");
        let outcome = self.build_outcome(reason.clone());
        self.emit(TurnEvent::TurnFinished { reason }).await?;
        Ok(outcome)
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
    async fn execute_tool_batch(
        &mut self,
        batch: ToolBatch,
    ) -> Result<Vec<ToolOutcome>, TurnError> {
        info!(
            step_index = batch.step_index,
            tool_count = batch.calls.len(),
            "tool batch prepared"
        );
        let mut join_set = JoinSet::new();
        let mut pending_permissions = Vec::new();
        let mut outcomes = vec![None; batch.calls.len()];
        let mut cancellation_requested = false;

        if self.consume_cancel_request()? {
            self.finish_cancelled().await?;
            return Ok(vec![]);
        }

        for (call_index, call) in batch.calls.iter().enumerate() {
            if self.consume_cancel_request()? {
                self.finish_cancelled().await?;
                return Ok(vec![]);
            }

            match self.authorizer.authorize(call.as_ref()).await? {
                AuthorizationDecision::Allow => {
                    self.spawn_tool_call(&mut join_set, call_index, Arc::clone(call));
                }
                AuthorizationDecision::Deny => {
                    let cid = call_id_arc(call.as_ref());
                    info!(call_id = %cid, "tool call denied");
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id: Arc::clone(&cid),
                        result: ToolOutcome::Denied,
                    })
                    .await?;
                    outcomes[call_index] = Some(ToolOutcome::Denied);
                }
                AuthorizationDecision::Ask(request) => {
                    info!(
                        request_id = %request.request_id,
                        tool_call_id = %request.tool_call_id,
                        "tool permission requested"
                    );
                    self.emit(TurnEvent::ToolCallPermissionRequested {
                        request: request.clone(),
                    })
                    .await?;
                    pending_permissions.push(PendingPermissionCall {
                        request,
                        call_index,
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
                                turn_id: self.seed.turn_id.clone(),
                            });
                        }
                        crate::TurnCommand::ResolvePermission { request_id, decision } => {
                            if let Some(index) = pending_permissions
                                .iter()
                                .position(|pending| pending.request.request_id == request_id)
                            {
                                let pending = pending_permissions.swap_remove(index);
                                info!(
                                    request_id = %request_id,
                                    tool_call_id = %pending.request.tool_call_id,
                                    decision = ?decision,
                                    "tool permission resolved"
                                );
                                self.emit(TurnEvent::ToolCallPermissionResolved {
                                    request_id: request_id.as_str().into(),
                                    decision: decision.clone(),
                                })
                                .await?;

                                match decision {
                                    PermissionDecision::Allow => {
                                        self.spawn_tool_call(
                                            &mut join_set,
                                            pending.call_index,
                                            Arc::clone(&batch.calls[pending.call_index]),
                                        );
                                    }
                                    PermissionDecision::Deny => {
                                        let cid = call_id_arc(batch.calls[pending.call_index].as_ref());
                                        self.emit(TurnEvent::ToolCallCompleted {
                                            call_id: Arc::clone(&cid),
                                            result: ToolOutcome::Denied,
                                        })
                                        .await?;
                                        outcomes[pending.call_index] = Some(ToolOutcome::Denied);
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
                    let (call_index, task_result) = result
                        .map_err(|err| TurnError::Runtime(format!("tool task join failed: {err}")))?;
                    let call = &batch.calls[call_index];
                    let cid = call_id_arc(call.as_ref());
                    info!(call_id = %cid, "tool call completed");
                    let outcome = map_tool_result(task_result);
                    self.emit(TurnEvent::ToolCallCompleted {
                        call_id: Arc::clone(&cid),
                        result: outcome.clone(),
                    })
                    .await?;
                    outcomes[call_index] = Some(outcome);
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

        let ordered = outcomes
            .into_iter()
            .map(|outcome| outcome.unwrap_or(ToolOutcome::Denied))
            .collect();

        Ok(ordered)
    }

    fn spawn_tool_call(
        &self,
        join_set: &mut JoinSet<(usize, ToolTaskResult)>,
        call_index: usize,
        call: SharedToolCall,
    ) {
        let tool_runner = Arc::clone(&self.tool_runner);
        let session_id = self.seed.session_id.clone();
        let turn_id = self.seed.turn_id.clone();
        let cancel_token = self.cancel_token.child_token();
        let timeout = self.options.tool_timeout;
        join_set.spawn(async move {
            let result = match tokio::time::timeout(
                timeout,
                tool_runner.execute(
                    call.as_ref().clone(),
                    ToolContext::new(session_id, turn_id, cancel_token),
                ),
            )
            .await
            {
                Ok(result) => ToolTaskResult::Completed(result),
                Err(_) => ToolTaskResult::TimedOut,
            };
            (call_index, result)
        });
    }

    async fn finish_cancelled(&mut self) -> Result<TurnOutcome, TurnError> {
        self.state = TurnState::Cancelled(TurnSummary {
            turn_id: self.seed.turn_id.clone(),
        });
        self.finish(TurnFinishReason::Cancelled).await
    }

    fn build_outcome(&self, finish_reason: TurnFinishReason) -> TurnOutcome {
        let transcript = self.transcript.to_vec();
        let final_output = transcript.iter().rev().find_map(|message| match message {
            TurnMessage::AssistantText { content } => Some(content.as_ref().to_owned()),
            _ => None,
        });

        TurnOutcome {
            turn_id: self.seed.turn_id.clone(),
            finish_reason,
            transcript,
            final_output,
        }
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

    fn streaming_step_mut(&mut self) -> Result<&mut ActiveLlmStep, TurnError> {
        match &mut self.state {
            TurnState::StreamingLlm(step) => Ok(step),
            _ => Err(TurnError::Runtime(
                "turn state invariant violated: expected streaming step".into(),
            )),
        }
    }

    fn take_streaming_step(&mut self) -> Result<ActiveLlmStep, TurnError> {
        match std::mem::replace(&mut self.state, TurnState::Ready) {
            TurnState::StreamingLlm(step) => Ok(step),
            _ => Err(TurnError::Runtime(
                "turn state invariant violated: missing streaming step".into(),
            )),
        }
    }
}

fn call_id_str(call: &argus_core::ToolCall) -> &str {
    match call {
        argus_core::ToolCall::FunctionCall { call_id, .. } => call_id,
        argus_core::ToolCall::Builtin(c) => &c.call_id,
        argus_core::ToolCall::Mcp(c) => &c.id,
    }
}

fn call_id_arc(call: &argus_core::ToolCall) -> Arc<str> {
    call_id_str(call).into()
}

fn tool_name_str(call: &argus_core::ToolCall) -> &str {
    match call {
        argus_core::ToolCall::FunctionCall { name, .. } => name,
        argus_core::ToolCall::Builtin(c) => c.builtin.canonical_name(),
        argus_core::ToolCall::Mcp(c) => c.name.as_deref().unwrap_or_default(),
    }
}

fn tool_name_arc(call: &argus_core::ToolCall) -> Arc<str> {
    tool_name_str(call).into()
}

fn outcome_to_content(outcome: &ToolOutcome) -> Arc<str> {
    match outcome {
        ToolOutcome::Success(v) => Arc::from(v.to_string()),
        ToolOutcome::Failed { message, .. } => Arc::clone(message),
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
            message: Arc::from(r.output.to_string()),
            retryable: false,
        },
        ToolTaskResult::Completed(Ok(r)) => ToolOutcome::Success(r.output),
        ToolTaskResult::Completed(Err(e)) => ToolOutcome::Failed {
            message: Arc::from(e.to_string()),
            retryable: false,
        },
        ToolTaskResult::TimedOut => ToolOutcome::TimedOut,
    }
}
