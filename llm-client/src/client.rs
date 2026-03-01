// llm-client/src/client.rs
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::adapter::{AdapterId, ProviderAdapter};
use crate::providers::anthropic::{AnthropicAdapter, AnthropicConfig};
use crate::providers::bigmodel::{BigModelAdapter, BigModelConfig};
use crate::providers::openai::{OpenAiAdapter, OpenAiConfig};
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
        LlmClientBuilder { registry: HashMap::new(), default_adapter: None }
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
        let adapter = self
            .registry
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
        let adapter = self
            .registry
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

    /// Register BigModel as the default adapter with explicit base URL and API key.
    pub fn with_default_bigmodel(
        mut self,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, LlmError> {
        self = self.with_bigmodel_adapter(base_url, api_key, HashMap::new())?;
        self.default_adapter = Some("bigmodel".to_string());
        Ok(self)
    }

    pub fn with_bigmodel_adapter(
        mut self,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self, LlmError> {
        let config = BigModelConfig {
            base_url: base_url.into(),
            api_key: api_key.into(),
            headers,
        };
        let adapter = Arc::new(BigModelAdapter::new(config)) as Arc<dyn ProviderAdapter>;
        self.registry.insert("bigmodel".to_string(), adapter);
        Ok(self)
    }

    pub fn with_openai_adapter(
        mut self,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self, LlmError> {
        let config = OpenAiConfig {
            base_url: base_url.into(),
            api_key: api_key.into(),
            headers,
        };
        let adapter = Arc::new(OpenAiAdapter::new(config)) as Arc<dyn ProviderAdapter>;
        self.registry.insert("openai".to_string(), adapter);
        Ok(self)
    }

    pub fn with_anthropic_adapter(
        mut self,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self, LlmError> {
        let config = AnthropicConfig {
            base_url: base_url.into(),
            api_key: api_key.into(),
            headers,
        };
        let adapter = Arc::new(AnthropicAdapter::new(config)) as Arc<dyn ProviderAdapter>;
        self.registry.insert("anthropic".to_string(), adapter);
        Ok(self)
    }

    /// Register BigModel as the default adapter using environment variables.
    /// Reads BIGMODEL_API_KEY and optionally BIGMODEL_BASE_URL.
    pub fn with_default_bigmodel_from_env(self) -> Result<Self, LlmError> {
        let api_key = std::env::var("BIGMODEL_API_KEY")
            .map_err(|_| LlmError::InvalidRequest { message: "BIGMODEL_API_KEY is required".to_string() })?;
        let base_url = std::env::var("BIGMODEL_BASE_URL")
            .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4".to_string());
        self.with_default_bigmodel(base_url, api_key)
    }

    pub fn build(self) -> Result<LlmClient, LlmError> {
        let default_adapter = self.default_adapter.ok_or_else(|| LlmError::InvalidRequest {
            message: "default adapter is required".to_string(),
        })?;

        if !self.registry.contains_key(&default_adapter) {
            return Err(LlmError::InvalidRequest {
                message: format!("default adapter '{}' not found in registry", default_adapter),
            });
        }

        Ok(LlmClient { registry: self.registry, default_adapter })
    }
}
