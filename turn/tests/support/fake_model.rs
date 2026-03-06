use async_trait::async_trait;
use argus_core::{FinishReason, ResponseEvent, ResponseStream, Usage};
use tokio::sync::mpsc;
use tokio::task;
use turn::{LlmRequestSnapshot, ModelRunner, TurnError};

pub struct FakeModelRunner {
    events: Vec<ResponseEvent>,
}

impl FakeModelRunner {
    pub fn new(events: Vec<ResponseEvent>) -> Self {
        Self { events }
    }
}

#[async_trait]
impl ModelRunner for FakeModelRunner {
    async fn start(&self, _request: LlmRequestSnapshot) -> Result<ResponseStream, TurnError> {
        let (tx, rx) = mpsc::channel(4);
        let events = self.events.clone();
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
        Self::new(vec![ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        }])
    }
}
