use std::collections::HashMap;

use turn::TurnController;

#[derive(Default)]
pub struct TurnManager {
    controllers: tokio::sync::Mutex<HashMap<String, TurnController>>,
}

impl TurnManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert(&self, turn_id: String, controller: TurnController) {
        self.controllers.lock().await.insert(turn_id, controller);
    }

    pub async fn take(&self, turn_id: &str) -> Option<TurnController> {
        self.controllers.lock().await.remove(turn_id)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::{sync::mpsc, task};
    use turn::{
        LlmStepRequest, ModelRunner, ToolAuthorizer, ToolRunner, TurnContext, TurnDriver,
        TurnError, TurnEvent, TurnFinishReason, TurnObserver,
    };

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn turn_manager_inserts_and_takes_controller_once() {
        let manager = TurnManager::new();
        let (handle, task) = TurnDriver::spawn(
            TurnContext {
                session_id: "session-1".into(),
                turn_id: "turn-1".into(),
                user_message: "hello".into(),
            },
            Arc::new(ImmediateStopModelRunner),
            Arc::new(NoopToolRunner),
            Arc::new(DenyAllAuthorizer),
            Arc::new(NoopObserver),
        );

        manager.insert("turn-1".into(), handle.controller()).await;

        assert!(manager.take("turn-1").await.is_some());
        assert!(manager.take("turn-1").await.is_none());

        while handle.next_event().await.is_some() {}
        task.await.unwrap().unwrap();
    }

    struct ImmediateStopModelRunner;

    #[async_trait]
    impl ModelRunner for ImmediateStopModelRunner {
        async fn start(
            &self,
            _request: LlmStepRequest,
        ) -> Result<argus_core::ResponseStream, TurnError> {
            let (tx, rx) = mpsc::channel(1);
            let producer = task::spawn(async move {
                tx.send(argus_core::ResponseEvent::Done {
                    reason: argus_core::FinishReason::Stop,
                    usage: Some(argus_core::Usage::zero()),
                })
                .await
                .unwrap();
            });

            Ok(argus_core::ResponseStream::from_parts(
                rx,
                producer.abort_handle(),
            ))
        }
    }

    struct NoopToolRunner;

    #[async_trait]
    impl ToolRunner for NoopToolRunner {
        async fn execute(
            &self,
            _call: argus_core::ToolCall,
            _ctx: tool::ToolContext,
        ) -> Result<tool::ToolResult, TurnError> {
            Ok(tool::ToolResult::ok(serde_json::json!({})))
        }
    }

    struct DenyAllAuthorizer;

    #[async_trait]
    impl ToolAuthorizer for DenyAllAuthorizer {
        async fn authorize(
            &self,
            _call: &argus_core::ToolCall,
        ) -> Result<turn::AuthorizationDecision, TurnError> {
            Ok(turn::AuthorizationDecision::Deny)
        }
    }

    struct NoopObserver;

    #[async_trait]
    impl TurnObserver for NoopObserver {
        async fn on_event(&self, event: &TurnEvent) -> Result<(), TurnError> {
            if matches!(
                event,
                TurnEvent::TurnFinished {
                    reason: TurnFinishReason::Completed
                }
            ) {
                return Ok(());
            }

            Ok(())
        }
    }
}
