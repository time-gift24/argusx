use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use argus_core::{FinishReason, ResponseEvent, ResponseStream, Usage};
use tokio::sync::{Mutex, mpsc};
use tokio::task;
use turn::{LlmRequestSnapshot, ModelRunner, TurnError};

pub struct FakeModelRunner {
    invocations: Arc<Mutex<VecDeque<Vec<ResponseEvent>>>>,
}

impl FakeModelRunner {
    pub fn new(invocations: Vec<Vec<ResponseEvent>>) -> Self {
        Self {
            invocations: Arc::new(Mutex::new(invocations.into())),
        }
    }
}

#[async_trait]
impl ModelRunner for FakeModelRunner {
    async fn start(&self, _request: LlmRequestSnapshot) -> Result<ResponseStream, TurnError> {
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
