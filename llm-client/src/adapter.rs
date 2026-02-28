// llm-client/src/adapter.rs
use async_trait::async_trait;

use crate::{LlmChunkStream, LlmError, LlmRequest, LlmResponse};

pub type AdapterId = String;

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn id(&self) -> &str;
    async fn chat(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    fn chat_stream(&self, req: LlmRequest) -> LlmChunkStream;
}
