# Desktop Runtime LLM Config Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 desktop chat 提供运行时（不持久化）LLM 配置能力，支持 BigModel/OpenAI/Anthropic 三 provider，并在无可用模型时禁用输入。

**Architecture:** 在 desktop-tauri 增加内存态 runtime config 与配置命令，前端 chat 页面新增右上角配置弹窗与动态模型可用性联动。将 provider+model（及 provider 连接参数）作为 turn 级参数向下传递到 agent-core/agent-turn，由 agent-turn 按 provider 路由到 llm-client 对应 adapter。

**Tech Stack:** Tauri v2, Rust workspace (`desktop`, `agent-core`, `agent-turn`, `llm-client`), Next.js 16, React 19, zustand, shadcn/ui

---

**Implementation Rules:** 执行时使用 @test-driven-development、@verification-before-completion、@rust-best-practices，按任务小步提交。

### Task 1: 建立 desktop 运行时配置模型与可用模型推导

**Files:**
- Create: `desktop/src-tauri/src/llm_runtime_config.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Test: `desktop/src-tauri/src/llm_runtime_config.rs` (module tests)

**Step 1: Write failing tests for normalization and availability**

```rust
#[test]
fn normalize_requires_api_key_base_url_and_models() {
    let cfg = LlmRuntimeConfig::default();
    let result = normalize_runtime_config(cfg);
    assert!(result.is_ok()); // default should be accepted but unavailable

    let bad = ProviderRuntimeConfig {
        api_key: "".into(),
        base_url: "https://api.example.com".into(),
        models: vec!["gpt-4o".into()],
        headers: vec![],
    };
    assert!(!bad.is_available());
}

#[test]
fn list_available_models_only_returns_available_provider_models() {
    let cfg = sample_runtime_config();
    let models = list_available_models(&cfg);
    assert_eq!(models, vec![
        AvailableModel { provider: ProviderId::OpenAi, model: "gpt-4o".into() },
        AvailableModel { provider: ProviderId::Anthropic, model: "claude-3-7-sonnet".into() },
    ]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop llm_runtime_config -- --nocapture`
Expected: FAIL with unresolved module/types.

**Step 3: Implement runtime config model and helper functions**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    Bigmodel,
    OpenAi,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRuntimeConfig {
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub headers: Vec<HeaderPair>,
}

impl ProviderRuntimeConfig {
    pub fn is_available(&self) -> bool {
        !self.api_key.trim().is_empty()
            && !self.base_url.trim().is_empty()
            && !self.models.is_empty()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop llm_runtime_config -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/llm_runtime_config.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add runtime llm config domain model"
```

### Task 2: 新增 Tauri 配置命令并接入 start_agent_turn 校验

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Test: `desktop/src-tauri/src/lib.rs` (new `#[cfg(test)]` tests for selection validation)

**Step 1: Write failing tests for provider/model selection validation**

```rust
#[test]
fn validate_turn_selection_rejects_unavailable_provider() {
    let cfg = LlmRuntimeConfig::default();
    let err = validate_turn_selection(&cfg, ProviderId::OpenAi, "gpt-4o").unwrap_err();
    assert!(err.contains("provider is not configured"));
}

#[test]
fn validate_turn_selection_rejects_unknown_model() {
    let cfg = sample_openai_only();
    let err = validate_turn_selection(&cfg, ProviderId::OpenAi, "gpt-5").unwrap_err();
    assert!(err.contains("model is not enabled"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop validate_turn_selection -- --nocapture`
Expected: FAIL with missing function.

**Step 3: Implement commands + payload changes + validation**

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartAgentTurnPayload {
    session_id: String,
    input: String,
    provider: ProviderId,
    model: String,
    attachments: Option<Vec<serde_json::Value>>,
}

#[tauri::command]
async fn set_llm_runtime_config(
    state: State<'_, AppState>,
    payload: LlmRuntimeConfig,
) -> Result<LlmRuntimeConfig, String> {
    let normalized = normalize_runtime_config(payload)?;
    *state.llm_runtime_config.write().await = normalized.clone();
    Ok(normalized)
}
```

**Step 4: Register commands and pass validated provider/model into TurnRequest**

Run: `cargo test -p desktop --lib -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add runtime config tauri commands and turn validation"
```

### Task 3: Chat API 封装与前端内存态配置 store

**Files:**
- Modify: `desktop/lib/api/chat.ts`
- Create: `desktop/lib/stores/llm-runtime-config-store.ts`
- Modify: `desktop/lib/stores/chat-store.ts`

**Step 1: Write failing type usage by switching `startAgentTurn` to require provider**

```ts
export interface StartAgentTurnPayload {
  sessionId: string;
  input: string;
  provider: "bigmodel" | "openai" | "anthropic";
  model: string;
  attachments?: unknown[];
}
```

**Step 2: Run type/lint checks (expect fail before call sites update)**

Run: `pnpm --dir desktop lint`
Expected: FAIL at `startAgentTurn` call sites missing `provider`.

**Step 3: Implement API wrappers + runtime config store**

```ts
export async function getLlmRuntimeConfig(): Promise<LlmRuntimeConfig> {
  return invoke("get_llm_runtime_config");
}

export const useLlmRuntimeConfigStore = create<LlmRuntimeConfigState>((set, get) => ({
  config: null,
  availableModels: [],
  selected: null,
  // no persist middleware: runtime-only by design
}));
```

**Step 4: Re-run checks**

Run: `pnpm --dir desktop lint`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/api/chat.ts desktop/lib/stores/llm-runtime-config-store.ts desktop/lib/stores/chat-store.ts
git commit -m "feat(desktop): add frontend runtime llm config api and store"
```

### Task 4: Chat 右上角配置按钮与动画、配置弹窗 UI

**Files:**
- Create: `desktop/components/features/chat/chat-runtime-config-dialog.tsx`
- Modify: `desktop/components/features/chat/chat-page.tsx`
- Modify: `desktop/app/globals.css`

**Step 1: Add UI contract first (Dialog props + callbacks), keep compile failing until wired**

```tsx
export interface ChatRuntimeConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}
```

**Step 2: Run lint to verify temporary fail points**

Run: `pnpm --dir desktop lint`
Expected: FAIL for missing imports/usages.

**Step 3: Implement animated top-right settings button + dialog form**

```tsx
<Button
  className="chat-config-trigger"
  onClick={() => setConfigDialogOpen(true)}
  size="icon"
  variant="outline"
>
  <Settings2 className="size-4" />
</Button>
```

```css
.chat-config-trigger {
  animation: llm-config-pulse 2.6s ease-in-out infinite;
}
@media (prefers-reduced-motion: reduce) {
  .chat-config-trigger { animation: none; }
}
```

**Step 4: Re-run lint**

Run: `pnpm --dir desktop lint`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/features/chat/chat-runtime-config-dialog.tsx desktop/components/features/chat/chat-page.tsx desktop/app/globals.css
git commit -m "feat(chat): add animated runtime llm config dialog entry"
```

### Task 5: PromptInput 动态模型源、红色告警与禁用态

**Files:**
- Modify: `desktop/components/features/chat/chat-prompt-input.tsx`
- Modify: `desktop/components/features/chat/chat-session-bar.tsx` (if props pass-through needed)
- Modify: `desktop/components/features/chat/session-badge.tsx` (only if visual consistency requires)

**Step 1: Introduce failing branch for unavailable models state**

```tsx
const hasAvailableModels = availableModels.length > 0;

if (!hasAvailableModels) {
  // TODO render red warning + disable prompt controls
}
```

**Step 2: Run lint/type checks (expect fail until all controls wired)**

Run: `pnpm --dir desktop lint`
Expected: FAIL for unused vars / missing branches.

**Step 3: Implement disabled prompt and red model indicator**

```tsx
<PromptInputTextarea disabled={!hasAvailableModels} placeholder={
  hasAvailableModels ? "Send a message..." : "Configure model providers first"
} />
<PromptInputSubmit disabled={!hasAvailableModels} status={hasAvailableModels ? status : "ready"} />
{!hasAvailableModels ? (
  <p className="text-xs text-red-500">No available models. Please configure provider settings.</p>
) : null}
```

**Step 4: Re-run lint**

Run: `pnpm --dir desktop lint`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/features/chat/chat-prompt-input.tsx desktop/components/features/chat/chat-session-bar.tsx desktop/components/features/chat/session-badge.tsx
git commit -m "feat(chat): disable prompt when no models and show warning state"
```

### Task 6: 扩展 agent-core 的 turn/model 请求结构

**Files:**
- Modify: `agent-core/src/model.rs`
- Modify: `agent-core/src/lib.rs` (re-export updates if needed)
- Test: `agent-core/tests/model_request_tools_test.rs`
- Modify: `agent-core/src/traits.rs` (compile-through)

**Step 1: Write failing test for new fields serialization**

```rust
#[test]
fn model_request_serializes_provider_and_model() {
    let req = ModelRequest {
        epoch: 1,
        provider: "openai".to_string(),
        model: "gpt-4o".to_string(),
        transcript: vec![],
        inputs: vec![InputEnvelope::user_text("hi")],
        tools: vec![],
    };
    let raw = serde_json::to_string(&req).unwrap();
    assert!(raw.contains("\"provider\":\"openai\""));
    assert!(raw.contains("\"model\":\"gpt-4o\""));
}
```

**Step 2: Run tests and verify fail**

Run: `cargo test -p agent-core --test model_request_tools_test -- --nocapture`
Expected: FAIL with missing fields.

**Step 3: Implement `TurnRequest` + `ModelRequest` fields**

```rust
pub struct TurnRequest {
    pub meta: SessionMeta,
    pub provider: String,
    pub model: String,
    pub initial_input: InputEnvelope,
    #[serde(default)]
    pub transcript: Vec<TranscriptItem>,
}

pub struct ModelRequest {
    pub epoch: u64,
    pub provider: String,
    pub model: String,
    pub transcript: Vec<TranscriptItem>,
    pub inputs: Vec<InputEnvelope>,
    #[serde(default)]
    pub tools: Vec<crate::tools::ToolSpec>,
}
```

**Step 4: Re-run tests**

Run: `cargo test -p agent-core --test model_request_tools_test -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-core/src/model.rs agent-core/src/lib.rs agent-core/src/traits.rs agent-core/tests/model_request_tools_test.rs
git commit -m "feat(agent-core): add provider and model fields to turn/model requests"
```

### Task 7: 贯通 agent-session / desktop 调用点以传递 provider+model

**Files:**
- Modify: `agent-session/src/session_runtime.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Build to capture failing call sites**

Run: `cargo test -p agent-session --lib -- --nocapture`
Expected: FAIL due to `TurnRequest` struct initialization missing `provider/model`.

**Step 2: Patch all TurnRequest constructors**

```rust
let request = TurnRequest {
    meta: SessionMeta::new(backend_session_id, turn_id.clone()),
    provider: payload.provider.as_str().to_string(),
    model: payload.model.clone(),
    initial_input: InputEnvelope::user_text(payload.input),
    transcript: Vec::new(),
};
```

**Step 3: Re-run tests**

Run: `cargo test -p agent-session --lib -- --nocapture`
Expected: PASS.

**Step 4: Commit**

```bash
git add agent-session/src/session_runtime.rs desktop/src-tauri/src/lib.rs
git commit -m "refactor(runtime): pass provider and model through turn request"
```

### Task 8: 改造 agent-turn adapter 为 provider 路由调用

**Files:**
- Modify: `agent-turn/src/adapters/bigmodel.rs`
- Modify: `agent-turn/src/effect.rs`
- Modify: `agent-turn/src/lib.rs` (export names if renamed)
- Test: `agent-turn/src/adapters/bigmodel.rs` (existing tests + new routing test)

**Step 1: Add failing routing test**

```rust
#[test]
fn convert_model_request_uses_request_provider_and_model() {
    let req = ModelRequest {
        epoch: 0,
        provider: "anthropic".to_string(),
        model: "claude-3-7-sonnet".to_string(),
        transcript: vec![],
        inputs: vec![InputEnvelope::user_text("hi")],
        tools: vec![],
    };
    let converted = convert_model_request(req, &BigModelAdapterConfig::default());
    assert_eq!(converted.model, "claude-3-7-sonnet");
}
```

**Step 2: Run targeted tests (expect fail)**

Run: `cargo test -p agent-turn adapters::bigmodel::tests -- --nocapture`
Expected: FAIL on missing `provider/model` handling.

**Step 3: Implement per-request routing to llm-client adapter id**

```rust
let provider = request.provider.clone();
let llm_request = convert_model_request(request, &self.config);
let mut stream = match client.chat_stream_with_adapter(provider, llm_request) {
    Ok(s) => s,
    Err(e) => { let _ = tx.send(Err(map_llm_error(e))); return; }
};
```

**Step 4: Re-run tests**

Run: `cargo test -p agent-turn adapters::bigmodel::tests -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/adapters/bigmodel.rs agent-turn/src/effect.rs agent-turn/src/lib.rs
git commit -m "feat(agent-turn): route model stream by request provider"
```

### Task 9: llm-client 增加按 adapter 调用能力 + OpenAI provider

**Files:**
- Modify: `llm-client/src/client.rs`
- Create: `llm-client/src/providers/openai.rs`
- Modify: `llm-client/src/providers/mod.rs`
- Test: `llm-client/tests/client_registry_test.rs`
- Test: `llm-client/tests/openai_adapter_test.rs`

**Step 1: Write failing tests for explicit adapter call and OpenAI success path**

```rust
#[tokio::test]
async fn chat_with_adapter_errors_for_missing_adapter() {
    let client = LlmClient::builder().with_default_bigmodel("http://x", "k").unwrap().build().unwrap();
    let req = sample_request();
    let err = client.chat_with_adapter("openai", req).await.unwrap_err();
    assert!(err.to_string().contains("adapter not found"));
}
```

**Step 2: Run tests and confirm fail**

Run: `cargo test -p llm-client --test client_registry_test -- --nocapture`
Expected: FAIL with method not found.

**Step 3: Implement client APIs + OpenAI adapter registration helper**

```rust
pub async fn chat_with_adapter(&self, id: impl AsRef<str>, req: LlmRequest) -> Result<LlmResponse, LlmError> { ... }
pub fn chat_stream_with_adapter(&self, id: impl AsRef<str>, req: LlmRequest) -> Result<LlmChunkStream, LlmError> { ... }

pub fn with_openai_adapter(
    mut self,
    base_url: impl Into<String>,
    api_key: impl Into<String>,
    headers: std::collections::HashMap<String, String>,
) -> Result<Self, LlmError> { ... }
```

**Step 4: Add OpenAI wiremock tests**

Run: `cargo test -p llm-client --test openai_adapter_test -- --nocapture`
Expected: PASS (non-stream + stream chunk mapping).

**Step 5: Commit**

```bash
git add llm-client/src/client.rs llm-client/src/providers/mod.rs llm-client/src/providers/openai.rs llm-client/tests/client_registry_test.rs llm-client/tests/openai_adapter_test.rs
git commit -m "feat(llm-client): add explicit adapter calls and openai provider"
```

### Task 10: llm-client 增加 Anthropic provider 并接入 desktop runtime config

**Files:**
- Create: `llm-client/src/providers/anthropic.rs`
- Modify: `llm-client/src/providers/mod.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Test: `llm-client/tests/anthropic_adapter_test.rs`
- Test: `desktop/src-tauri/src/lib.rs` (provider setup tests)

**Step 1: Write failing anthropic adapter tests**

```rust
#[tokio::test]
async fn anthropic_adapter_maps_content_text() {
    // wiremock response: content[0].text -> LlmResponse.output_text
}
```

**Step 2: Run tests to verify fail**

Run: `cargo test -p llm-client --test anthropic_adapter_test -- --nocapture`
Expected: FAIL with missing module.

**Step 3: Implement anthropic adapter + desktop builder wiring for all providers**

```rust
let mut builder = LlmClient::builder();
if let Some(cfg) = runtime_config.provider(ProviderId::OpenAi).filter(|p| p.is_available()) {
    builder = builder.with_openai_adapter(cfg.base_url.clone(), cfg.api_key.clone(), cfg.header_map())?;
}
if let Some(cfg) = runtime_config.provider(ProviderId::Anthropic).filter(|p| p.is_available()) {
    builder = builder.with_anthropic_adapter(cfg.base_url.clone(), cfg.api_key.clone(), cfg.header_map())?;
}
```

**Step 4: Run crate and workspace verification**

Run: `cargo test -p llm-client --tests -- --nocapture`
Expected: PASS.

Run: `cargo test -p desktop --lib -- --nocapture`
Expected: PASS.

Run: `cargo test -p agent-core --tests -- --nocapture`
Expected: PASS.

Run: `cargo test -p agent-turn --tests -- --nocapture`
Expected: PASS.

Run: `pnpm --dir desktop lint`
Expected: PASS.

**Step 5: Commit**

```bash
git add llm-client/src/providers/anthropic.rs llm-client/src/providers/mod.rs desktop/src-tauri/src/lib.rs llm-client/tests/anthropic_adapter_test.rs
git commit -m "feat(llm-client): add anthropic provider and desktop runtime wiring"
```

### Task 11: 最终验收与文档同步

**Files:**
- Modify: `desktop/docs/plans/2026-03-01-llm-chat-frontend.md` (append runtime-config delta)
- Modify: `docs/plans/2026-03-01-desktop-llm-runtime-config-design.md` (mark implemented checklist)

**Step 1: Manual acceptance checklist**

1. 清空环境变量启动 desktop。
2. 进入 `/chat`，验证右上角配置按钮动画存在。
3. 未配置时验证 PromptInput 禁用 + 红色文案。
4. 配置 OpenAI/Anthropic/BigModel 任一后，模型可选并可发送消息。
5. 重启应用后配置丢失。

**Step 2: Record verification evidence in docs**

```markdown
- [x] No env var startup works
- [x] Runtime config dialog appears
- [x] Input disabled when no models
- [x] Three providers configurable
- [x] Non-persistent behavior confirmed
```

**Step 3: Final checks**

Run: `git status --short`
Expected: only intended files staged/clean after commit.

**Step 4: Commit**

```bash
git add desktop/docs/plans/2026-03-01-llm-chat-frontend.md docs/plans/2026-03-01-desktop-llm-runtime-config-design.md
git commit -m "docs: record runtime llm config implementation verification"
```
