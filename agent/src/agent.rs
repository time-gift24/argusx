use std::sync::Arc;

use crate::error::AgentFacadeError;
use crate::types::{AgentStream, AgentStreamEvent, ChatResponse, ChatTurnStatus};
use agent_core::{
    new_id, InputEnvelope, RunStreamEvent, Runtime, SessionMeta, TurnRequest, UiThreadEvent,
};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct Agent<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    runtime: Arc<agent_session::SessionRuntime<L, agent_tool::AgentToolRuntime>>,
}

impl<L> Agent<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    pub(crate) fn new(
        runtime: Arc<agent_session::SessionRuntime<L, agent_tool::AgentToolRuntime>>,
    ) -> Self {
        Self { runtime }
    }

    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> Result<String, AgentFacadeError> {
        self.runtime
            .create_session(user_id, title)
            .await
            .map_err(AgentFacadeError::from_anyhow)
    }

    pub async fn list_sessions(
        &self,
        filter: agent_session::SessionFilter,
    ) -> Result<Vec<agent_core::SessionInfo>, AgentFacadeError> {
        self.runtime
            .list_sessions(filter)
            .await
            .map_err(AgentFacadeError::from_anyhow)
    }

    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<agent_core::SessionInfo>, AgentFacadeError> {
        self.runtime
            .get_session(&session_id.to_string())
            .await
            .map_err(AgentFacadeError::from_anyhow)
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), AgentFacadeError> {
        self.runtime
            .delete_session(&session_id.to_string())
            .await
            .map_err(AgentFacadeError::from_anyhow)
    }

    pub async fn chat(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<ChatResponse, AgentFacadeError> {
        if session_id.trim().is_empty() {
            return Err(AgentFacadeError::InvalidInput {
                message: "session_id cannot be empty".to_string(),
            });
        }

        let text = message.trim();
        if text.is_empty() {
            return Err(AgentFacadeError::InvalidInput {
                message: "message cannot be empty".to_string(),
            });
        }

        let turn_id = new_id();
        let request = TurnRequest {
            meta: SessionMeta::new(session_id.to_string(), turn_id.clone()),
            provider: "bigmodel".to_string(),
            model: "glm-5".to_string(),
            initial_input: InputEnvelope::user_text(text),
            transcript: Vec::new(),
        };

        let streams = self
            .runtime
            .run_turn(request)
            .await
            .map_err(AgentFacadeError::from_agent_error)?;

        let mut run = streams.run;
        let mut ui = streams.ui;
        let mut terminal: Option<ChatResponse> = None;
        let mut fallback_text = String::new();

        let mut run_open = true;
        let mut ui_open = true;
        while run_open || ui_open {
            tokio::select! {
                event = run.next(), if run_open => {
                    match event {
                        Some(RunStreamEvent::TurnDone {
                            turn_id,
                            final_message,
                            usage,
                            ..
                        }) => {
                            terminal = Some(ChatResponse {
                                turn_id,
                                status: ChatTurnStatus::Done,
                                final_message,
                                input_tokens: usage.input_tokens,
                                output_tokens: usage.output_tokens,
                            });
                        }
                        Some(RunStreamEvent::TurnFailed {
                            turn_id,
                            cancelled,
                            usage,
                            ..
                        }) => {
                            terminal = Some(ChatResponse {
                                turn_id,
                                status: if cancelled {
                                    ChatTurnStatus::Cancelled
                                } else {
                                    ChatTurnStatus::Failed
                                },
                                final_message: None,
                                input_tokens: usage.input_tokens,
                                output_tokens: usage.output_tokens,
                            });
                        }
                        Some(_) => {}
                        None => run_open = false,
                    }
                }
                event = ui.next(), if ui_open => {
                    match event {
                        Some(UiThreadEvent::MessageDelta { delta, .. }) => fallback_text.push_str(&delta),
                        Some(_) => {}
                        None => ui_open = false,
                    }
                }
            }
        }

        if let Some(mut response) = terminal {
            if response.final_message.is_none() && !fallback_text.is_empty() {
                response.final_message = Some(fallback_text);
            }
            return Ok(response);
        }

        Err(AgentFacadeError::Execution {
            message: "turn ended without terminal event".to_string(),
        })
    }

    pub async fn chat_stream(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<AgentStream, AgentFacadeError> {
        if session_id.trim().is_empty() {
            return Err(AgentFacadeError::InvalidInput {
                message: "session_id cannot be empty".to_string(),
            });
        }

        let text = message.trim();
        if text.is_empty() {
            return Err(AgentFacadeError::InvalidInput {
                message: "message cannot be empty".to_string(),
            });
        }

        let turn_id = new_id();
        let request = TurnRequest {
            meta: SessionMeta::new(session_id.to_string(), turn_id),
            provider: "bigmodel".to_string(),
            model: "glm-5".to_string(),
            initial_input: InputEnvelope::user_text(text),
            transcript: Vec::new(),
        };

        let streams = self
            .runtime
            .run_turn(request)
            .await
            .map_err(AgentFacadeError::from_agent_error)?;

        let mut run = streams.run;
        let mut ui = streams.ui;
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut run_open = true;
            let mut ui_open = true;

            while run_open || ui_open {
                tokio::select! {
                    event = run.next(), if run_open => {
                        match event {
                            Some(event) => {
                                if tx.send(AgentStreamEvent::Run(event)).is_err() {
                                    break;
                                }
                            }
                            None => run_open = false,
                        }
                    }
                    event = ui.next(), if ui_open => {
                        match event {
                            Some(event) => {
                                if tx.send(AgentStreamEvent::Ui(event)).is_err() {
                                    break;
                                }
                            }
                            None => ui_open = false,
                        }
                    }
                }
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    pub async fn inject_input(
        &self,
        turn_id: &str,
        input: agent_core::InputEnvelope,
    ) -> Result<(), AgentFacadeError> {
        if turn_id.trim().is_empty() {
            return Err(AgentFacadeError::InvalidInput {
                message: "turn_id cannot be empty".to_string(),
            });
        }

        self.runtime
            .inject_input(turn_id, input)
            .await
            .map_err(AgentFacadeError::from_agent_error)
    }

    pub async fn cancel_turn(
        &self,
        turn_id: &str,
        reason: Option<String>,
    ) -> Result<(), AgentFacadeError> {
        if turn_id.trim().is_empty() {
            return Err(AgentFacadeError::InvalidInput {
                message: "turn_id cannot be empty".to_string(),
            });
        }

        self.runtime
            .cancel_turn(turn_id, reason)
            .await
            .map_err(AgentFacadeError::from_agent_error)
    }
}
