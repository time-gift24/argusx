use argus_core::ResponseStream;
use async_trait::async_trait;

use crate::{TurnError, transcript::TurnMessageSnapshot};

#[derive(Debug, Clone, PartialEq)]
pub struct LlmStepRequest {
    pub session_id: String,
    pub turn_id: String,
    pub step_index: u32,
    pub messages: TurnMessageSnapshot,
    pub allow_tools: bool,
}

#[async_trait]
pub trait ModelRunner: Send + Sync {
    async fn start(&self, request: LlmStepRequest) -> Result<ResponseStream, TurnError>;
}
