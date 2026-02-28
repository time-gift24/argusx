# Llm Client Provider Encapsulation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace public `BigModel*` APIs with a provider-agnostic `LlmClient` facade and migrate all callers in one pass.

**Architecture:** Introduce a new public facade (`LlmClient`, `LlmClientBuilder`, `LlmRequest`, `LlmResponse`, `LlmChunk`) plus a registry-based adapter abstraction (`ProviderAdapter`). Keep BigModel implementation internal (`pub(crate)`), and map internal provider payloads to generic types. Migrate gateway and agent callers to generic APIs, then seal `providers` from public exports.

**Tech Stack:** Rust, tokio, futures-core stream trait, reqwest, axum, wiremock, async-trait.

---

## Preconditions

- Work in a dedicated worktree before executing this plan.
- Relevant skills to follow during execution: `@superpowers:test-driven-development`, `@superpowers:verification-before-completion`, `@rust-router`, `@domain-web`.
- Do not include unrelated working tree changes in commits.

### Task 1: Introduce Provider-Agnostic Public Types

**Files:**
- Create: `llm-client/src/types.rs`
- Modify: `llm-client/src/lib.rs`
- Test: `llm-client/tests/types_roundtrip_test.rs`

**Step 1: Write the failing test**

```rust
// llm-client/tests/types_roundtrip_test.rs
use llm_client::{LlmChunk, LlmRequest, LlmRole};

#[test]
fn llm_request_and_chunk_are_constructible() {
    let req = LlmRequest {
        model: "glm-5".to_string(),
        messages: vec![llm_client::LlmMessage {
            role: LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: true,
        max_tokens: Some(128),
        temperature: Some(0.7),
        top_p: Some(0.9),
    };

    assert!(req.stream);

    let chunk = LlmChunk {
        delta_text: Some("hi".to_string()),
        delta_reasoning: None,
        finish_reason: None,
        usage: None,
    };
    assert_eq!(chunk.delta_text.as_deref(), Some("hi"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client llm_request_and_chunk_are_constructible`
Expected: FAIL with unresolved imports (`LlmRequest`, `LlmChunk`, `LlmRole`).

**Step 3: Write minimal implementation**

```rust
// llm-client/src/types.rs
use std::pin::Pin;

use futures_core::Stream;
use serde::{Deserialize, Serialize};

use crate::LlmError;

pub type LlmChunkStream = Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send + 'static>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub stream: bool,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub id: String,
    pub model: String,
    pub output_text: String,
    pub usage: Option<LlmUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChunk {
    pub delta_text: Option<String>,
    pub delta_reasoning: Option<String>,
    pub finish_reason: Option<String>,
    pub usage: Option<LlmUsage>,
}
```

```rust
// llm-client/src/lib.rs (exports)
pub mod types;

pub use types::{
    LlmChunk, LlmChunkStream, LlmMessage, LlmRequest, LlmResponse, LlmRole, LlmUsage,
};
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client llm_request_and_chunk_are_constructible`
Expected: PASS.

**Step 5: Commit**

```bash
git add llm-client/src/types.rs llm-client/src/lib.rs llm-client/tests/types_roundtrip_test.rs
git commit -m "feat(llm-client): add provider-agnostic request and stream types"
```

### Task 2: Add Adapter Abstraction and Client Facade

**Files:**
- Create: `llm-client/src/adapter.rs`
- Create: `llm-client/src/client.rs`
- Modify: `llm-client/src/error.rs`
- Modify: `llm-client/src/lib.rs`
- Test: `llm-client/tests/client_registry_test.rs`

**Step 1: Write the failing test**

```rust
// llm-client/tests/client_registry_test.rs
use llm_client::{LlmClient, LlmRequest, LlmRole, LlmMessage};

#[tokio::test]
async fn build_client_without_default_adapter_fails() {
    let result = LlmClient::builder().build();
    assert!(result.is_err());
}

#[tokio::test]
async fn calling_unknown_adapter_fails() {
    let client = LlmClient::builder()
        .default_adapter("missing")
        .build()
        .unwrap_err();
    assert!(client.to_string().contains("default adapter"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client build_client_without_default_adapter_fails`
Expected: FAIL because `LlmClient`/builder do not exist.

**Step 3: Write minimal implementation**

```rust
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
```

```rust
// llm-client/src/client.rs
use std::collections::HashMap;
use std::sync::Arc;

use crate::adapter::{AdapterId, ProviderAdapter};
use crate::{LlmChunkStream, LlmError, LlmRequest, LlmResponse};

pub struct LlmClient {
    registry: HashMap<AdapterId, Arc<dyn ProviderAdapter>>,
    default_adapter: AdapterId,
}

pub struct LlmClientBuilder {
    registry: HashMap<AdapterId, Arc<dyn ProviderAdapter>>,
    default_adapter: Option<AdapterId>,
}

impl LlmClient {
    pub fn builder() -> LlmClientBuilder {
        LlmClientBuilder { registry: HashMap::new(), default_adapter: None }
    }

    pub async fn chat(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.registry
            .get(&self.default_adapter)
            .ok_or_else(|| LlmError::InvalidRequest { message: "default adapter not found".to_string() })?
            .chat(req)
            .await
    }

    pub fn chat_stream(&self, req: LlmRequest) -> Result<LlmChunkStream, LlmError> {
        let adapter = self
            .registry
            .get(&self.default_adapter)
            .ok_or_else(|| LlmError::InvalidRequest { message: "default adapter not found".to_string() })?;
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
        let default_adapter = self.default_adapter.ok_or_else(|| LlmError::InvalidRequest {
            message: "default adapter is required".to_string(),
        })?;

        Ok(LlmClient { registry: self.registry, default_adapter })
    }
}
```

```rust
// llm-client/src/lib.rs exports
mod adapter;
mod client;

pub use adapter::{AdapterId, ProviderAdapter};
pub use client::{LlmClient, LlmClientBuilder};
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client build_client_without_default_adapter_fails`
Expected: PASS.

**Step 5: Commit**

```bash
git add llm-client/src/adapter.rs llm-client/src/client.rs llm-client/src/lib.rs llm-client/tests/client_registry_test.rs
git commit -m "feat(llm-client): add adapter registry and llm client facade"
```

### Task 3: Implement Internal BigModel Adapter and Mapping

**Files:**
- Modify: `llm-client/src/providers/bigmodel.rs`
- Modify: `llm-client/src/providers/mod.rs`
- Create: `llm-client/src/mapping/bigmodel.rs`
- Modify: `llm-client/src/lib.rs`
- Test: `llm-client/tests/integration_test.rs`

**Step 1: Write the failing test**

```rust
// llm-client/tests/integration_test.rs (replace provider-specific ctor usage)
#[tokio::test]
async fn facade_can_chat_via_bigmodel_adapter() {
    // existing wiremock setup
    let client = llm_client::LlmClient::builder()
        .with_default_bigmodel(mock_server.uri(), "test-key")
        .unwrap()
        .build()
        .unwrap();

    let req = llm_client::LlmRequest {
        model: "glm-5".to_string(),
        messages: vec![llm_client::LlmMessage {
            role: llm_client::LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
    };

    let res = client.chat(req).await.unwrap();
    assert_eq!(res.model, "glm-5");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client facade_can_chat_via_bigmodel_adapter`
Expected: FAIL because builder helper and adapter mapping do not exist.

**Step 3: Write minimal implementation**

```rust
// llm-client/src/providers/mod.rs
pub(crate) mod bigmodel;
```

```rust
// llm-client/src/mapping/bigmodel.rs
pub fn to_bigmodel_request(req: &crate::LlmRequest) -> bigmodel_api::ChatRequest { /* full field mapping */ }
pub fn to_llm_response(resp: &bigmodel_api::ChatResponse) -> crate::LlmResponse { /* map id/model/text/usage */ }
pub fn to_llm_chunk(chunk: &bigmodel_api::ChatResponseChunk) -> crate::LlmChunk { /* map delta text/reasoning/finish */ }
```

```rust
// llm-client/src/providers/bigmodel.rs
pub(crate) struct BigModelAdapter { /* internal config + existing http clients */ }

#[async_trait::async_trait]
impl crate::ProviderAdapter for BigModelAdapter {
    fn id(&self) -> &str { "bigmodel" }
    async fn chat(&self, req: crate::LlmRequest) -> Result<crate::LlmResponse, crate::LlmError> { /* call existing logic + mapping */ }
    fn chat_stream(&self, req: crate::LlmRequest) -> crate::LlmChunkStream { /* existing stream logic + mapping */ }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client integration_test`
Expected: PASS.

**Step 5: Commit**

```bash
git add llm-client/src/providers/mod.rs llm-client/src/providers/bigmodel.rs llm-client/src/mapping/bigmodel.rs llm-client/tests/integration_test.rs
git commit -m "feat(llm-client): make bigmodel adapter internal and map to generic types"
```

### Task 4: Add Builder Helpers for BigModel Registration

**Files:**
- Modify: `llm-client/src/client.rs`
- Modify: `llm-client/src/lib.rs`
- Test: `llm-client/tests/client_builder_bigmodel_test.rs`

**Step 1: Write the failing test**

```rust
// llm-client/tests/client_builder_bigmodel_test.rs
#[test]
fn with_default_bigmodel_from_env_requires_api_key() {
    std::env::remove_var("BIGMODEL_API_KEY");
    let result = llm_client::LlmClient::builder().with_default_bigmodel_from_env();
    assert!(result.is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client with_default_bigmodel_from_env_requires_api_key`
Expected: FAIL because helper is missing.

**Step 3: Write minimal implementation**

```rust
// llm-client/src/client.rs impl LlmClientBuilder
pub fn with_default_bigmodel(
    self,
    base_url: impl Into<String>,
    api_key: impl Into<String>,
) -> Result<Self, LlmError> { /* instantiate internal BigModelAdapter, register it, set default */ }

pub fn with_default_bigmodel_from_env(self) -> Result<Self, LlmError> {
    let api_key = std::env::var("BIGMODEL_API_KEY")
        .map_err(|_| LlmError::InvalidRequest { message: "BIGMODEL_API_KEY is required".to_string() })?;
    let base_url = std::env::var("BIGMODEL_BASE_URL")
        .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4".to_string());
    self.with_default_bigmodel(base_url, api_key)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client with_default_bigmodel_from_env_requires_api_key`
Expected: PASS.

**Step 5: Commit**

```bash
git add llm-client/src/client.rs llm-client/src/lib.rs llm-client/tests/client_builder_bigmodel_test.rs
git commit -m "feat(llm-client): add bigmodel builder helpers without exposing provider types"
```

### Task 5: Migrate `llm-gateway` to Generic `LlmClient`

**Files:**
- Modify: `llm-gateway/src/main.rs`
- Modify: `llm-gateway/src/lib.rs`
- Test: `llm-gateway/src/lib.rs` (existing tests)

**Step 1: Write/adjust failing tests**

```rust
// keep existing tests, but construct state with LlmClient builder helper
let client = llm_client::LlmClient::builder()
    .with_default_bigmodel(mock_server.uri(), "test-key")
    .unwrap()
    .build()
    .unwrap();
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-gateway`
Expected: FAIL due to removed `llm_client::providers::*` imports.

**Step 3: Write minimal implementation**

```rust
// llm-gateway/src/main.rs
let client = llm_client::LlmClient::builder()
    .with_default_bigmodel_from_env()?
    .build()?;
```

```rust
// llm-gateway/src/lib.rs
#[derive(Clone)]
pub struct GatewayState {
    pub client: Arc<llm_client::LlmClient>,
}

// chat path uses client.chat(req).await and client.chat_stream(req)?
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-gateway`
Expected: PASS (non-stream and stream tests both green).

**Step 5: Commit**

```bash
git add llm-gateway/src/main.rs llm-gateway/src/lib.rs
git commit -m "refactor(llm-gateway): use generic llm client facade"
```

### Task 6: Migrate `agent-turn` Adapter to Generic Stream Chunks

**Files:**
- Modify: `agent-turn/src/adapters/bigmodel.rs`
- Test: `agent-turn/src/adapters/bigmodel.rs` (existing unit tests)

**Step 1: Write/adjust failing test**

```rust
// update an existing stream chunk test to consume llm_client::LlmChunk
assert!(matches!(item, ModelOutputEvent::TextDelta { .. }));
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn stream_chunk_emits_text_and_reasoning_deltas`
Expected: FAIL due to old `ChatResponseChunk`-specific assumptions.

**Step 3: Write minimal implementation**

```rust
// agent-turn/src/adapters/bigmodel.rs
use llm_client::{LlmChunk, LlmClient, LlmMessage, LlmRequest, LlmRole};

pub struct BigModelModelAdapter {
    client: Arc<LlmClient>,
    config: BigModelAdapterConfig,
}

// convert_model_request now builds LlmRequest
// emit_chunk now accepts LlmChunk
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn bigmodel`
Expected: PASS for adapter tests.

**Step 5: Commit**

```bash
git add agent-turn/src/adapters/bigmodel.rs
git commit -m "refactor(agent-turn): consume generic llm request and chunk types"
```

### Task 7: Migrate CLI Entry Points to New Builder API

**Files:**
- Modify: `agent-cli/src/main.rs`
- Modify: `agent-turn-cli/src/main.rs`
- Test: `agent-cli/tests/cli_help.rs`
- Test: `agent-turn-cli/src/main.rs` (existing smoke/unit tests)

**Step 1: Write/adjust failing usage test**

```rust
// ensure binaries still initialize when BIGMODEL_API_KEY is present in env-driven path
```

**Step 2: Run test to verify it fails**

Run: `cargo check -p agent-cli -p agent-turn-cli`
Expected: FAIL due to removed provider imports.

**Step 3: Write minimal implementation**

```rust
// agent-cli/src/main.rs and agent-turn-cli/src/main.rs
let client = std::sync::Arc::new(
    llm_client::LlmClient::builder()
        .with_default_bigmodel_from_env()?
        .build()?,
);
```

**Step 4: Run test to verify it passes**

Run: `cargo check -p agent-cli -p agent-turn-cli && cargo test -p agent-cli -p agent-turn-cli`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-cli/src/main.rs agent-turn-cli/src/main.rs
git commit -m "refactor(cli): initialize llm facade via builder helpers"
```

### Task 8: Seal Provider API and Final Verification

**Files:**
- Modify: `llm-client/src/lib.rs`
- Modify: `llm-client/src/providers/mod.rs`
- Modify: `llm-client/tests/integration_test.rs` (if still importing providers)
- Optional docs: `llm-client/src/lib.rs` crate docs example

**Step 1: Write the failing public API check**

```bash
rg -n "llm_client::providers|BigModelHttpClient|BigModelConfig" agent-cli agent-turn agent-turn-cli llm-gateway
```

Expected (for this step): non-zero matches before sealing.

**Step 2: Run check to verify it fails (precondition)**

Run: same `rg` command above.
Expected: output contains old provider imports.

**Step 3: Write minimal implementation**

```rust
// llm-client/src/lib.rs
// remove: pub mod providers;
// export only facade + generic types
```

```rust
// llm-client/src/providers/mod.rs
pub(crate) mod bigmodel;
```

Update crate docs example to use:

```rust
let client = llm_client::LlmClient::builder()
    .with_default_bigmodel_from_env()?
    .build()?;
```

**Step 4: Run full verification to confirm completion**

Run:

```bash
rg -n "llm_client::providers|BigModelHttpClient|BigModelConfig" agent-cli agent-turn agent-turn-cli llm-gateway
cargo test -p llm-client -p llm-gateway -p agent-turn -p agent-cli -p agent-turn-cli
cargo check --workspace
```

Expected:
- `rg` finds zero matches outside `llm-client` internal provider implementation.
- all tests PASS.
- workspace check PASS.

**Step 5: Commit**

```bash
git add llm-client/src/lib.rs llm-client/src/providers/mod.rs llm-client/tests/integration_test.rs
git commit -m "refactor(llm-client): hide provider module and expose facade-only api"
```

## Completion Checklist

- No external crate imports `llm_client::providers::*`.
- `llm-gateway` and agent stacks run on `LlmClient` facade only.
- Streaming behavior and `[DONE]` semantics preserved.
- Error mapping still stable (`LlmError` -> HTTP status in gateway).
- All targeted tests and workspace checks are green.
