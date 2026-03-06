use async_trait::async_trait;

use crate::{TurnError, TurnEvent};

#[async_trait]
pub trait TurnObserver: Send + Sync {
    async fn on_event(&self, event: &TurnEvent) -> Result<(), TurnError>;
}
