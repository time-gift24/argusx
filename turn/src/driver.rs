use std::sync::Arc;

use argus_core::{FinishReason, ResponseEvent};
use futures::StreamExt;
use tokio::{
    sync::mpsc,
    task::{self, JoinHandle},
};

use crate::{
    LlmRequestSnapshot, ModelRunner, ToolAuthorizer, ToolRunner, TurnContext, TurnError,
    TurnEvent, TurnFailure, TurnFinishReason, TurnHandle, TurnState, TurnSummary,
    state::ActiveLlmStep,
};

pub struct TurnDriver {
    context: TurnContext,
    model: Arc<dyn ModelRunner>,
    _tool_runner: Arc<dyn ToolRunner>,
    _authorizer: Arc<dyn ToolAuthorizer>,
    observer: Arc<dyn crate::TurnObserver>,
    state: TurnState,
    _command_rx: mpsc::Receiver<crate::TurnCommand>,
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
            _tool_runner: tool_runner,
            _authorizer: authorizer,
            observer,
            _command_rx: command_rx,
            event_tx,
        };

        let task = task::spawn(async move { driver.run().await });
        (handle, task)
    }

    async fn run(mut self) -> Result<(), TurnError> {
        self.emit(TurnEvent::TurnStarted).await?;

        self.state = TurnState::StreamingLlm(ActiveLlmStep { step_index: 0 });
        let request = LlmRequestSnapshot {
            session_id: self.context.session_id.clone(),
            turn_id: self.context.turn_id.clone(),
            input_text: self.context.user_message.clone(),
        };

        let mut stream = self.model.start(request).await?;
        while let Some(event) = stream.next().await {
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
                ResponseEvent::Done { reason, .. } => {
                    let summary = TurnSummary {
                        turn_id: self.context.turn_id.clone(),
                    };
                    let finish_reason = match reason {
                        FinishReason::Cancelled => TurnFinishReason::Cancelled,
                        _ => TurnFinishReason::Completed,
                    };
                    self.state = match finish_reason {
                        TurnFinishReason::Completed => TurnState::Completed(summary),
                        TurnFinishReason::Cancelled => TurnState::Cancelled(summary),
                        TurnFinishReason::Failed => TurnState::Failed(TurnFailure {
                            message: "turn failed".into(),
                        }),
                    };
                    self.emit(TurnEvent::TurnFinished {
                        reason: finish_reason,
                    })
                    .await?;
                    return Ok(());
                }
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
                | ResponseEvent::ReasoningDone(_)
                | ResponseEvent::ToolDone(_) => {}
            }
        }

        Err(TurnError::Runtime(
            "model stream ended before a terminal event".into(),
        ))
    }

    async fn emit(&self, event: TurnEvent) -> Result<(), TurnError> {
        self.observer.on_event(&event).await?;
        self.event_tx
            .send(event)
            .await
            .map_err(|_| TurnError::Runtime("turn event receiver dropped".into()))
    }
}
