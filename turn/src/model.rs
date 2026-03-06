use async_trait::async_trait;
use argus_core::ResponseStream;

use crate::TurnError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRequestSnapshot {
    pub session_id: String,
    pub turn_id: String,
    pub input_text: String,
}

#[async_trait]
pub trait ModelRunner: Send + Sync {
    async fn start(&self, request: LlmRequestSnapshot) -> Result<ResponseStream, TurnError>;
}
