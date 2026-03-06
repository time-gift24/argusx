use async_trait::async_trait;
use argus_core::{ResponseEvent, ResponseStream, Usage};
use tokio::sync::mpsc;
use tokio::task;
use turn::{LlmRequestSnapshot, ModelRunner, TurnError};

pub struct FakeModelRunner;

#[async_trait]
impl ModelRunner for FakeModelRunner {
    async fn start(&self, _request: LlmRequestSnapshot) -> Result<ResponseStream, TurnError> {
        let (tx, rx) = mpsc::channel(4);
        let producer = task::spawn(async move {
            tx.send(ResponseEvent::Done {
                reason: argus_core::FinishReason::Stop,
                usage: Some(Usage::zero()),
            })
            .await
            .unwrap();
        });

        Ok(ResponseStream::from_parts(rx, producer.abort_handle()))
    }
}
