use async_trait::async_trait;
use turn::{TurnError, TurnEvent, TurnObserver};

pub struct FakeObserver;

#[async_trait]
impl TurnObserver for FakeObserver {
    async fn on_event(&self, _event: &TurnEvent) -> Result<(), TurnError> {
        Ok(())
    }
}
