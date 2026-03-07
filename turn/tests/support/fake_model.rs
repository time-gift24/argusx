use std::{collections::VecDeque, sync::Arc};

use argus_core::{FinishReason, ResponseEvent, ResponseStream, Usage};
use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};
use tokio::task;
use turn::{LlmStepRequest, ModelRunner, TurnError};

pub struct FakeModelRunner {
    invocations: Arc<Mutex<VecDeque<Vec<ResponseEvent>>>>,
    received: Arc<Mutex<Vec<LlmStepRequest>>>,
}

impl FakeModelRunner {
    pub fn new(invocations: Vec<Vec<ResponseEvent>>) -> Self {
        Self {
            invocations: Arc::new(Mutex::new(invocations.into())),
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn received_requests(&self) -> Vec<LlmStepRequest> {
        self.received.lock().await.clone()
    }
}

#[async_trait]
impl ModelRunner for FakeModelRunner {
    async fn start(&self, request: LlmStepRequest) -> Result<ResponseStream, TurnError> {
        // NOTE: received.push and invocations.pop_front are not atomic.
        // This is safe in single-driver tests; do not share across concurrent drivers.
        self.received.lock().await.push(request);
        let (tx, rx) = mpsc::channel(4);
        let events = self
            .invocations
            .lock()
            .await
            .pop_front()
            .ok_or_else(|| TurnError::Runtime("no fake model invocation remaining".into()))?;
        let producer = task::spawn(async move {
            for event in events {
                tx.send(event).await.unwrap();
            }
        });
        Ok(ResponseStream::from_parts(rx, producer.abort_handle()))
    }
}

impl Default for FakeModelRunner {
    fn default() -> Self {
        Self::new(vec![vec![ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        }]])
    }
}
