// llm-client/src/client.rs
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::adapter::{AdapterId, ProviderAdapter};
use crate::{LlmChunkStream, LlmError, LlmRequest, LlmResponse};

pub struct LlmClient {
    registry: HashMap<AdapterId, Arc<dyn ProviderAdapter>>,
    default_adapter: AdapterId,
}

impl fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LlmClient")
            .field("default_adapter", &self.default_adapter)
            .finish()
    }
}

pub struct LlmClientBuilder {
    registry: HashMap<AdapterId, Arc<dyn ProviderAdapter>>,
    default_adapter: Option<AdapterId>,
}

impl fmt::Debug for LlmClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LlmClientBuilder")
            .field("default_adapter", &self.default_adapter)
            .finish()
    }
}

impl LlmClient {
    pub fn builder() -> LlmClientBuilder {
        LlmClientBuilder {
            registry: HashMap::new(),
            default_adapter: None,
        }
    }

    pub async fn chat(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.chat_with_adapter(&self.default_adapter, req).await
    }

    pub fn chat_stream(&self, req: LlmRequest) -> Result<LlmChunkStream, LlmError> {
        self.chat_stream_with_adapter(&self.default_adapter, req)
    }

    pub async fn chat_with_adapter(
        &self,
        adapter_id: impl AsRef<str>,
        req: LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        let adapter =
            self.registry
                .get(adapter_id.as_ref())
                .ok_or_else(|| LlmError::InvalidRequest {
                    message: format!("adapter '{}' not found", adapter_id.as_ref()),
                })?;
        adapter.chat(req).await
    }

    pub fn chat_stream_with_adapter(
        &self,
        adapter_id: impl AsRef<str>,
        req: LlmRequest,
    ) -> Result<LlmChunkStream, LlmError> {
        let adapter =
            self.registry
                .get(adapter_id.as_ref())
                .ok_or_else(|| LlmError::InvalidRequest {
                    message: format!("adapter '{}' not found", adapter_id.as_ref()),
                })?;
        Ok(adapter.chat_stream(req))
    }
}

impl LlmClientBuilder {
    pub fn register_adapter(mut self, adapter: Arc<dyn ProviderAdapter>) -> Self {
        self.registry.insert(adapter.id().to_string(), adapter);
        self
    }

    pub fn default_adapter(mut self, id: impl Into<String>) -> Self {
        self.default_adapter = Some(id.into());
        self
    }

    pub fn build(self) -> Result<LlmClient, LlmError> {
        let default_adapter = self
            .default_adapter
            .ok_or_else(|| LlmError::InvalidRequest {
                message: "default adapter is required".to_string(),
            })?;

        if !self.registry.contains_key(&default_adapter) {
            return Err(LlmError::InvalidRequest {
                message: format!(
                    "default adapter '{}' not found in registry",
                    default_adapter
                ),
            });
        }

        Ok(LlmClient {
            registry: self.registry,
            default_adapter,
        })
    }
}
