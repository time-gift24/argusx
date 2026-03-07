# Provider Settings Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为桌面应用增加右上角 provider 配置入口，支持将一个 `Z.ai` 配置和多个 `OpenAI-compatible` profile 保存到 SQLite，并以密文形式存储 API key，同时让 chat 运行时读取全局默认 profile。

**Architecture:** 在 Tauri 侧新增 provider settings 存储、加密与命令层，前端新增全局 settings dialog 管理 profile。聊天运行时优先从 SQLite 读取默认 profile，不存在时回退到现有环境变量。

**Tech Stack:** Rust (Tauri v2, rusqlite, provider crate), TypeScript (Next.js 16, React 19, Vitest), 系统安全存储

---

### Task 1: 后端 provider settings 数据模型与 SQLite schema

**Files:**
- Create: `desktop/src-tauri/src/provider_settings/mod.rs`
- Create: `desktop/src-tauri/src/provider_settings/model.rs`
- Create: `desktop/src-tauri/src/provider_settings/store.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/Cargo.toml`
- Test: `desktop/src-tauri/tests/provider_settings_store_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn store_enforces_single_default_profile() {
    let store = test_store();
    let first = sample_create("OpenRouter", true);
    let second = sample_create("Local vLLM", true);

    store.save(first).unwrap();
    let err = store.save(second).unwrap_err();

    assert!(err.to_string().contains("default"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop provider_settings_store_test -- --nocapture`
Expected: FAIL because store module does not exist yet

**Step 3: Write minimal implementation**

```rust
pub struct ProviderProfileStore {
    conn: Mutex<Connection>,
}

impl ProviderProfileStore {
    pub fn new(path: &Path) -> Result<Self, ProviderSettingsError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}
```

Include:
- `provider_profiles` table
- partial unique index for `is_default = 1`
- request/record structs for create, update, summary
- `provider_kind` enum with `zai` and `openai_compatible`

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop provider_settings_store_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/provider_settings desktop/src-tauri/src/lib.rs desktop/src-tauri/Cargo.toml desktop/src-tauri/tests/provider_settings_store_test.rs
git commit -m "feat: add provider settings sqlite store"
```

---

### Task 2: API key 加密与系统安全存储包装

**Files:**
- Create: `desktop/src-tauri/src/provider_settings/crypto.rs`
- Modify: `desktop/src-tauri/src/provider_settings/mod.rs`
- Modify: `desktop/src-tauri/Cargo.toml`
- Test: `desktop/src-tauri/tests/provider_settings_crypto_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn encrypt_round_trip_recovers_original_api_key() {
    let vault = TestVault::new();
    let sealed = vault.encrypt("sk-test-123").unwrap();

    assert_ne!(sealed.ciphertext, b"sk-test-123");
    assert_eq!(vault.decrypt(&sealed).unwrap(), "sk-test-123");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop provider_settings_crypto_test -- --nocapture`
Expected: FAIL because crypto module does not exist yet

**Step 3: Write minimal implementation**

```rust
pub trait SecretVault {
    fn encrypt(&self, plaintext: &str) -> Result<EncryptedSecret, ProviderSettingsError>;
    fn decrypt(&self, sealed: &EncryptedSecret) -> Result<String, ProviderSettingsError>;
}
```

Implement:
- DEK generation
- DEK load/store through system secure storage abstraction
- symmetric encryption for API key
- test double vault for unit tests

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop provider_settings_crypto_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/provider_settings/crypto.rs desktop/src-tauri/src/provider_settings/mod.rs desktop/src-tauri/Cargo.toml desktop/src-tauri/tests/provider_settings_crypto_test.rs
git commit -m "feat: add encrypted provider secret storage"
```

---

### Task 3: Provider settings service 与 Tauri commands

**Files:**
- Create: `desktop/src-tauri/src/provider_settings/service.rs`
- Create: `desktop/src-tauri/src/provider_settings/commands.rs`
- Modify: `desktop/src-tauri/src/provider_settings/mod.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Test: `desktop/src-tauri/tests/provider_settings_commands_test.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn save_profile_keeps_existing_api_key_when_blank() {
    let app = test_app_state();
    let saved = save_profile(&app, create_request("OpenRouter", Some("sk-old"))).await.unwrap();
    let updated = save_profile(&app, update_request(&saved.id, Some("OpenRouter 2"), None)).await.unwrap();

    let decrypted = app.settings_service().load_secret(&updated.id).unwrap();
    assert_eq!(decrypted, "sk-old");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop provider_settings_commands_test -- --nocapture`
Expected: FAIL because commands do not exist yet

**Step 3: Write minimal implementation**

```rust
#[tauri::command]
pub async fn save_provider_profile(
    state: State<'_, AppState>,
    input: SaveProviderProfileInput,
) -> Result<ProviderProfileSummary, String> {
    state.provider_settings.save(input).map_err(|err| err.to_string())
}
```

Implement:
- list/save/delete/set_default/test_connection commands
- business errors for default deletion and decryption failures
- singleton constraint for `Z.ai`
- service glue between store and vault

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop provider_settings_commands_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/provider_settings desktop/src-tauri/src/lib.rs desktop/src-tauri/tests/provider_settings_commands_test.rs
git commit -m "feat: expose provider settings tauri commands"
```

---

### Task 4: Chat runtime 优先读取 SQLite 默认 profile

**Files:**
- Modify: `desktop/src-tauri/src/chat/model.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/src/provider_settings/service.rs`
- Test: `desktop/src-tauri/tests/chat_model_test.rs`
- Test: `desktop/src-tauri/tests/provider_settings_runtime_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn sqlite_default_profile_overrides_environment_variables() {
    let service = configured_settings_service();
    let env = env_config("https://env.example", "env-model", "sk-env");

    let resolved = ChatProviderConfig::resolve(Some(&service), env).unwrap();

    assert_eq!(resolved.base_url, "https://sqlite.example");
    assert_eq!(resolved.model, "sqlite-model");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop provider_settings_runtime_test -- --nocapture`
Expected: FAIL because runtime still only reads env vars

**Step 3: Write minimal implementation**

```rust
impl ProviderModelRunner {
    pub fn from_sources(
        settings: Option<&ProviderSettingsService>,
    ) -> Result<Self, TurnError> {
        let config = settings
            .and_then(|service| service.load_default_runtime_config().transpose())
            .transpose()?
            .flatten()
            .unwrap_or_else(Self::env_runtime_config);

        Self::new(config.model, config.provider)
    }
}
```

Implement:
- runtime config resolver
- `provider_kind -> Dialect` mapping for `Z.ai` and `OpenAI-compatible`
- explicit error when no sqlite default and no env fallback exist
- no behavior change for replay mode tests

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop chat_model_test provider_settings_runtime_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/model.rs desktop/src-tauri/src/lib.rs desktop/src-tauri/src/provider_settings/service.rs desktop/src-tauri/tests/chat_model_test.rs desktop/src-tauri/tests/provider_settings_runtime_test.rs
git commit -m "feat: load chat provider config from sqlite default profile"
```

---

### Task 5: 前端 IPC client 与 Provider Settings Dialog

**Files:**
- Create: `desktop/lib/provider-settings.ts`
- Create: `desktop/lib/provider-settings.test.ts`
- Create: `desktop/components/settings/provider-settings-dialog.tsx`
- Create: `desktop/components/settings/provider-profile-form.tsx`
- Modify: `desktop/components/layouts/app-layout.tsx`
- Test: `desktop/components/settings/provider-settings-dialog.test.tsx`
- Test: `desktop/app/page.test.tsx`

**Step 1: Write the failing test**

```tsx
it("opens provider settings dialog from header button", async () => {
  render(<AppLayout><div>content</div></AppLayout>);

  await user.click(screen.getByRole("button", { name: /provider 配置/i }));

  expect(screen.getByRole("dialog", { name: /provider 配置/i })).toBeInTheDocument();
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop test -- --runInBand components/settings/provider-settings-dialog.test.tsx app/page.test.tsx`
Expected: FAIL because button and dialog do not exist yet

**Step 3: Write minimal implementation**

```ts
export async function listProviderProfiles(): Promise<ProviderProfileSummary[]> {
  return invoke("list_provider_profiles");
}
```

Implement:
- IPC helpers for list/save/delete/set_default/test_connection
- dialog shell and list rendering
- explicit `Z.ai` entry plus `OpenAI-compatible` create flow
- create/edit form
- default toggle
- error toast / inline error handling

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop test -- --runInBand lib/provider-settings.test.ts components/settings/provider-settings-dialog.test.tsx app/page.test.tsx`
Expected: PASS

**Step 5: Commit**

```bash
git add desktop/lib/provider-settings.ts desktop/lib/provider-settings.test.ts desktop/components/settings/provider-settings-dialog.tsx desktop/components/settings/provider-profile-form.tsx desktop/components/layouts/app-layout.tsx desktop/components/settings/provider-settings-dialog.test.tsx desktop/app/page.test.tsx
git commit -m "feat: add provider settings dialog in app header"
```

---

### Task 6: 前后端联调、失败路径与回归验证

**Files:**
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/app/chat/page.test.tsx`
- Modify: `desktop/src-tauri/tests/provider_settings_commands_test.rs`
- Modify: `desktop/src-tauri/tests/provider_settings_runtime_test.rs`
- Test: `desktop/app/chat/page.test.tsx`

**Step 1: Write the failing test**

```tsx
it("shows configuration error when no default profile and no env fallback exist", async () => {
  mockStartTurnReject("provider configuration missing");
  render(<ChatPage />);

  await submitPrompt("hello");

  expect(screen.getByText(/provider configuration missing/i)).toBeInTheDocument();
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop test -- --runInBand app/chat/page.test.tsx`
Expected: FAIL because configuration errors are not surfaced in the chat UI

**Step 3: Write minimal implementation**

```tsx
try {
  await startTurn(input);
} catch (error) {
  setError(readableMessage(error));
}
```

Implement:
- front-end rendering for configuration failure
- regression coverage for default profile lifecycle
- final wiring verification between header settings and chat runtime

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop test -- --runInBand app/chat/page.test.tsx`
Expected: PASS

**Step 5: Commit**

```bash
git add desktop/app/chat/page.tsx desktop/app/chat/page.test.tsx desktop/src-tauri/tests/provider_settings_commands_test.rs desktop/src-tauri/tests/provider_settings_runtime_test.rs
git commit -m "feat: surface provider configuration errors in chat"
```

---

### Task 7: 全量验证

**Files:**
- Modify: `desktop/src-tauri/Cargo.toml`
- Modify: `desktop/package.json`

**Step 1: Run Rust tests**

Run: `cargo test -p desktop -- --nocapture`
Expected: PASS

**Step 2: Run telemetry regression**

Run: `cargo test -p telemetry -- --nocapture`
Expected: PASS

**Step 3: Run frontend tests**

Run: `pnpm --dir desktop test -- --runInBand`
Expected: PASS

**Step 4: Run static validation**

Run: `cargo check -p desktop`
Expected: PASS

Run: `git diff --check`
Expected: no output

**Step 5: Commit**

```bash
git add desktop telemetry
git commit -m "feat: add persisted provider settings for desktop chat"
```
