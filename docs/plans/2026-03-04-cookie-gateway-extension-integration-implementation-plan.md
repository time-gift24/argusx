# Cookie Gateway Extension Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver a production-usable Phase 1 cookie-gateway integration in desktop: first-run setup flow, extension package export, stable gateway/extension status detection, and safe state persistence.

**Architecture:** Implement backend in two layers: a testable cookie setup service/state module and thin Tauri command wrappers. In frontend, add a dedicated setup route and a client-side guard inside `AppLayout` (not root layout). Ship extension packaging through Tauri resources with deterministic zip build and add targeted security/performance hardening from the revised design.

**Tech Stack:** Rust (`tauri v2`, `tokio`, `serde`, `reqwest`), TypeScript/React (`Next.js 16`, `@tauri-apps/api/core`), Vitest + Testing Library, shell packaging (`bash`, `zip`).

---

## Execution Rules

- Reference skills during implementation: `@using-git-worktrees`, `@test-driven-development`, `@verification-before-completion`, `@systematic-debugging`.
- Keep changes task-scoped and commit after each task.
- Prefer focused test commands first, then broader smoke runs.
- Do not break existing desktop chat runtime behavior while integrating setup flow.

---

### Task 0: Isolated Worktree + Baseline Verification

**Files:**
- No source files in this task.

**Step 1: Create isolated worktree**

Run:
```bash
git worktree add .worktrees/codex/cookie-gateway-setup -b codex/cookie-gateway-setup
```
Expected: new worktree created and branch checked out.

**Step 2: Enter worktree and verify clean state**

Run:
```bash
cd .worktrees/codex/cookie-gateway-setup
git status --short
```
Expected: no output.

**Step 3: Run baseline targeted tests**

Run:
```bash
cargo test -p cookie-gateway -- --nocapture
cargo test -p desktop -- --nocapture
pnpm --dir desktop test -- lib/api/chat.test.ts components/layouts/theme-toggle.test.tsx
```
Expected: baseline should pass (or capture existing failures before new changes).

**Step 4: Create checkpoint commit**

Run:
```bash
git commit --allow-empty -m "chore: start cookie gateway extension integration"
```
Expected: empty checkpoint commit exists.

---

### Task 1: Fix Extension Packaging Script (TDD)

**Files:**
- Modify: `cookie-gateway/chrome-extension/build-extension.sh`
- Test: `cookie-gateway/chrome-extension/build-extension.sh` (script smoke via shell)

**Step 1: Add a failing script smoke check**

Run:
```bash
bash cookie-gateway/chrome-extension/build-extension.sh
```
Expected: currently fails because the script is malformed (undefined vars/invalid commands).

**Step 2: Rewrite script to deterministic minimal packaging**

Replace script with:
```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
DIST="$ROOT/dist"
ZIP_PATH="$DIST/chrome-extension.zip"

mkdir -p "$DIST"
rm -f "$ZIP_PATH"

cd "$ROOT"
zip -r "$ZIP_PATH" manifest.json background.js popup.html
echo "Extension packaged: $ZIP_PATH"
```

**Step 3: Run script and verify artifact**

Run:
```bash
bash cookie-gateway/chrome-extension/build-extension.sh
test -f cookie-gateway/chrome-extension/dist/chrome-extension.zip
unzip -l cookie-gateway/chrome-extension/dist/chrome-extension.zip
```
Expected: PASS, zip exists, and contains `manifest.json`, `background.js`, `popup.html`.

**Step 4: Commit script fix**

Run:
```bash
git add cookie-gateway/chrome-extension/build-extension.sh
git commit -m "fix(cookie-gateway): repair extension packaging script"
```

---

### Task 2: Expose Gateway Connection/Fetch Accessors (TDD)

**Files:**
- Modify: `cookie-gateway/src/command_bus.rs`
- Modify: `cookie-gateway/src/lib.rs`
- Modify: `cookie-gateway/tests/tool_test.rs`

**Step 1: Add failing tests for `CookieGateway` accessors**

In `cookie-gateway/tests/tool_test.rs`, add:
```rust
#[tokio::test]
async fn cookie_gateway_is_extension_connected_false_by_default() {
    let gateway = cookie_gateway::CookieGateway::new(cookie_gateway::CookieStore::new());
    assert!(!gateway.is_extension_connected().await);
}

#[tokio::test]
async fn cookie_gateway_fetch_cookies_returns_unavailable_without_client() {
    let gateway = cookie_gateway::CookieGateway::new(cookie_gateway::CookieStore::new());
    let result = gateway
        .fetch_cookies("api.company.com", std::time::Duration::from_secs(1))
        .await;
    assert!(matches!(
        result,
        Err(cookie_gateway::error::CookieGatewayError::ExtensionClientUnavailable)
    ));
}
```

**Step 2: Run tests and confirm fail**

Run:
```bash
cargo test -p cookie-gateway cookie_gateway_is_extension_connected_false_by_default -- --nocapture
cargo test -p cookie-gateway cookie_gateway_fetch_cookies_returns_unavailable_without_client -- --nocapture
```
Expected: FAIL because methods do not exist yet.

**Step 3: Implement minimal backend accessors**

In `cookie-gateway/src/command_bus.rs`, add:
```rust
impl GatewayCommandBus {
    pub async fn is_connected(&self) -> bool {
        self.connection.read().await.is_some()
    }
}
```

In `cookie-gateway/src/lib.rs`, add:
```rust
impl CookieGateway {
    pub async fn is_extension_connected(&self) -> bool {
        self.state.command_bus.is_connected().await
    }

    pub async fn fetch_cookies(
        &self,
        domain: &str,
        refresh_after: std::time::Duration,
    ) -> Result<crate::tool::CookieFetchOutput, crate::error::CookieGatewayError> {
        self.state.cookie_fetch_tool.fetch(domain, refresh_after).await
    }
}
```

**Step 4: Re-run targeted and full crate tests**

Run:
```bash
cargo test -p cookie-gateway cookie_gateway_is_extension_connected_false_by_default -- --nocapture
cargo test -p cookie-gateway cookie_gateway_fetch_cookies_returns_unavailable_without_client -- --nocapture
cargo test -p cookie-gateway -- --nocapture
```
Expected: PASS.

**Step 5: Commit gateway accessor changes**

Run:
```bash
git add cookie-gateway/src/command_bus.rs cookie-gateway/src/lib.rs cookie-gateway/tests/tool_test.rs
git commit -m "feat(cookie-gateway): expose extension connection and fetch accessors"
```

---

### Task 3: Add Cookie Setup State Persistence Module (TDD)

**Files:**
- Create: `desktop/src-tauri/src/cookie_setup_state.rs`
- Modify: `desktop/src-tauri/src/lib.rs` (module declaration and usage)
- Test: `desktop/src-tauri/src/cookie_setup_state.rs` (unit tests inside module)

**Step 1: Write failing state persistence tests**

In new `cookie_setup_state.rs`, add tests first:
```rust
use std::fs;
use tempfile::tempdir;

#[test]
fn default_state_is_not_started() {
    let s = CookieSetupState::default();
    assert_eq!(s.phase, CookieSetupPhase::NotStarted);
    assert_eq!(s.version, 1);
}

#[test]
fn save_then_load_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("cookie_setup_state.json");
    let state = CookieSetupState {
        phase: CookieSetupPhase::Skipped,
        updated_at: "2026-03-04T00:00:00Z".to_string(),
        skipped_reason: Some("later".to_string()),
        version: 1,
    };

    save_cookie_setup_state(&path, &state).expect("save state");
    let loaded = load_cookie_setup_state(&path);
    assert_eq!(loaded.phase, CookieSetupPhase::Skipped);
    assert_eq!(loaded.skipped_reason.as_deref(), Some("later"));
    assert_eq!(loaded.version, 1);
}

#[test]
fn corrupted_file_falls_back_to_default() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("cookie_setup_state.json");
    fs::write(&path, "{not json").expect("write broken file");

    let loaded = load_cookie_setup_state(&path);
    assert_eq!(loaded.phase, CookieSetupPhase::NotStarted);
    assert_eq!(loaded.version, 1);
}
```

**Step 2: Run test and confirm fail**

Run:
```bash
cargo test -p desktop cookie_setup_state -- --nocapture
```
Expected: FAIL before module implementation.

**Step 3: Implement state types + atomic load/save**

Implement in `cookie_setup_state.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CookieSetupPhase { NotStarted, WaitingExtension, Connected, Completed, Skipped }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CookieSetupState {
    pub phase: CookieSetupPhase,
    pub updated_at: String,
    pub skipped_reason: Option<String>,
    pub version: u32,
}

pub fn load_cookie_setup_state(path: &std::path::Path) -> CookieSetupState {
    let bytes = match std::fs::read(path) {
        Ok(v) => v,
        Err(_) => return CookieSetupState::default(),
    };
    serde_json::from_slice::<CookieSetupState>(&bytes).unwrap_or_default()
}

pub fn save_cookie_setup_state(
    path: &std::path::Path,
    state: &CookieSetupState,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("missing parent dir")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let bytes = serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}
```

Minimal save algorithm:
1. `serde_json::to_vec_pretty`
2. write to `path.with_extension("json.tmp")`
3. `std::fs::rename(tmp, path)`

**Step 4: Re-run tests**

Run:
```bash
cargo test -p desktop cookie_setup_state -- --nocapture
```
Expected: PASS.

**Step 5: Commit state module**

Run:
```bash
git add desktop/src-tauri/src/cookie_setup_state.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add cookie setup state persistence module"
```

---

### Task 4: Introduce Backend Service Layer for Setup Flow (TDD)

**Files:**
- Create: `desktop/src-tauri/src/cookie_setup_service.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Test: `desktop/src-tauri/src/cookie_setup_service.rs` (unit tests)

**Step 1: Add failing service tests**

In `cookie_setup_service.rs`, add tests first:
```rust
use crate::cookie_setup_state::{CookieSetupPhase, CookieSetupState};

#[tokio::test]
async fn status_reports_gateway_down_when_health_false() {
    let state = CookieSetupState::default();
    let status = CookieGatewayStatus {
        gateway_running: false,
        extension_connected: false,
        setup_phase: state.phase.clone(),
    };
    assert!(!status.gateway_running);
    assert!(!status.extension_connected);
    assert_eq!(status.setup_phase, CookieSetupPhase::NotStarted);
}

#[tokio::test]
async fn complete_sets_phase_completed() {
    let mut state = CookieSetupState {
        phase: CookieSetupPhase::WaitingExtension,
        updated_at: "2026-03-04T00:00:00Z".to_string(),
        skipped_reason: None,
        version: 1,
    };
    transition_to_completed(&mut state);
    assert_eq!(state.phase, CookieSetupPhase::Completed);
    assert_eq!(state.skipped_reason, None);
}

#[tokio::test]
async fn skip_sets_phase_skipped_with_reason() {
    let mut state = CookieSetupState::default();
    transition_to_skipped(&mut state, Some("later".to_string()));
    assert_eq!(state.phase, CookieSetupPhase::Skipped);
    assert_eq!(state.skipped_reason.as_deref(), Some("later"));
}
```

**Step 2: Run tests and confirm fail**

Run:
```bash
cargo test -p desktop cookie_setup_service -- --nocapture
```
Expected: FAIL before service implementation.

**Step 3: Implement service with reusable `reqwest::Client`**

Service skeleton:
```rust
pub struct CookieSetupService {
    pub setup_state_path: std::path::PathBuf,
    pub health_client: reqwest::Client,
}

impl CookieSetupService {
    pub async fn get_status(
        &self,
        gateway: &cookie_gateway::CookieGateway,
        state: &CookieSetupState,
    ) -> CookieGatewayStatus {
        let gateway_running = check_gateway_health(&self.health_client).await;
        let extension_connected = gateway.is_extension_connected().await;
        CookieGatewayStatus {
            gateway_running,
            extension_connected,
            setup_phase: state.phase.clone(),
        }
    }

    pub fn complete(&self, state: &mut CookieSetupState) -> Result<(), String> {
        transition_to_completed(state);
        save_cookie_setup_state(&self.setup_state_path, state)
    }

    pub fn skip(&self, reason: Option<String>, state: &mut CookieSetupState) -> Result<(), String> {
        transition_to_skipped(state, reason);
        save_cookie_setup_state(&self.setup_state_path, state)
    }
}
```

Implement helper:
```rust
pub async fn check_gateway_health(client: &reqwest::Client) -> bool {
    client.get("http://127.0.0.1:3456/health")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
```

**Step 4: Re-run tests**

Run:
```bash
cargo test -p desktop cookie_setup_service -- --nocapture
```
Expected: PASS.

**Step 5: Commit service layer**

Run:
```bash
git add desktop/src-tauri/src/cookie_setup_service.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add cookie setup service layer"
```

---

### Task 5: Add Tauri Commands and Wire AppState (TDD)

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Test: `desktop/src-tauri/src/lib.rs` (`#[cfg(test)]` additions)

**Step 1: Add failing command-level tests for transitions**

Add tests in `lib.rs` for pure transition helpers:
```rust
use crate::cookie_setup_state::{CookieSetupPhase, CookieSetupState};

#[test]
fn complete_transition_is_idempotent() {
    let mut s = CookieSetupState {
        phase: CookieSetupPhase::Completed,
        updated_at: "2026-03-04T00:00:00Z".to_string(),
        skipped_reason: None,
        version: 1,
    };
    transition_to_completed(&mut s);
    assert_eq!(s.phase, CookieSetupPhase::Completed);
}

#[test]
fn skip_does_not_map_to_completed() {
    let mut s = CookieSetupState::default();
    transition_to_skipped(&mut s, Some("later".to_string()));
    assert_eq!(s.phase, CookieSetupPhase::Skipped);
    assert_ne!(s.phase, CookieSetupPhase::Completed);
}
```

**Step 2: Run targeted desktop tests**

Run:
```bash
cargo test -p desktop complete_transition_is_idempotent -- --nocapture
cargo test -p desktop skip_does_not_map_to_completed -- --nocapture
```
Expected: FAIL before helper/command integration.

**Step 3: Implement commands and register invoke handler**

Add new commands in `lib.rs`:
```rust
#[tauri::command]
async fn get_cookie_gateway_status(state: State<'_, AppState>) -> Result<CookieGatewayStatus, String> {
    let setup = state.cookie_setup_state.read().await.clone();
    Ok(state.cookie_setup_service.get_status(&state.cookie_gateway, &setup).await)
}

#[tauri::command]
async fn get_cookie_setup_state(state: State<'_, AppState>) -> Result<CookieSetupState, String> {
    Ok(state.cookie_setup_state.read().await.clone())
}

#[tauri::command]
async fn complete_cookie_setup(state: State<'_, AppState>) -> Result<(), String> {
    let mut setup = state.cookie_setup_state.write().await;
    state.cookie_setup_service.complete(&mut setup)
}

#[tauri::command]
async fn skip_cookie_setup(reason: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let mut setup = state.cookie_setup_state.write().await;
    state.cookie_setup_service.skip(reason, &mut setup)
}

#[tauri::command]
async fn export_cookie_extension_bundle(app: AppHandle) -> Result<String, String> {
    let src = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("chrome-extension.zip");
    let dst = app
        .path()
        .download_dir()
        .map_err(|e| e.to_string())?
        .join("argusx-cookie-extension.zip");
    std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
    Ok(dst.to_string_lossy().to_string())
}
```

Wire new fields in `AppState`:
```rust
cookie_setup_state: Arc<RwLock<CookieSetupState>>,
cookie_setup_service: Arc<CookieSetupService>,
```

Register in `invoke_handler(...)`:
```rust
.invoke_handler(tauri::generate_handler![
    get_cookie_gateway_status,
    get_cookie_setup_state,
    complete_cookie_setup,
    skip_cookie_setup,
    export_cookie_extension_bundle,
])
```

**Step 4: Re-run backend tests**

Run:
```bash
cargo test -p desktop -- --nocapture
```
Expected: PASS.

**Step 5: Commit Tauri command integration**

Run:
```bash
git add desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add cookie setup tauri commands and app state wiring"
```

---

### Task 6: Wire Tauri Resource Packaging (Build Script + Config) (TDD-ish Smoke)

**Files:**
- Modify: `desktop/src-tauri/build.rs`
- Modify: `desktop/src-tauri/tauri.conf.json`
- Test: build smoke (commands)

**Step 1: Add build smoke command (expected to fail before fixes)**

Run:
```bash
cargo build -p desktop
test -f desktop/src-tauri/resources/chrome-extension.zip
```
Expected: first command may pass, second command fails because resource zip is not produced yet.

**Step 2: Implement build resource copy in `build.rs`**

Use:
```rust
fn main() {
    tauri_build::build();
    println!("cargo:rerun-if-changed=../../cookie-gateway/chrome-extension");

    let extension_dir = std::path::PathBuf::from("../../cookie-gateway/chrome-extension");
    let script = extension_dir.join("build-extension.sh");
    let status = std::process::Command::new("bash")
        .arg(script)
        .current_dir(&extension_dir)
        .status()
        .expect("failed to run extension build script");
    if !status.success() {
        panic!("extension build script failed");
    }

    let from = extension_dir.join("dist/chrome-extension.zip");
    let to_dir = std::path::PathBuf::from("resources");
    let to = to_dir.join("chrome-extension.zip");
    std::fs::create_dir_all(&to_dir).expect("create resources dir");
    std::fs::copy(from, to).expect("copy extension zip");
}
```

**Step 3: Add resource entry in `tauri.conf.json`**

Ensure:
```json
"bundle": {
  "resources": [
    "resources/chrome-extension.zip"
  ]
}
```

**Step 4: Re-run build + artifact check**

Run:
```bash
cargo build -p desktop
test -f desktop/src-tauri/resources/chrome-extension.zip
```
Expected: PASS.

**Step 5: Commit packaging wiring**

Run:
```bash
git add desktop/src-tauri/build.rs desktop/src-tauri/tauri.conf.json desktop/src-tauri/resources/chrome-extension.zip
git commit -m "build(desktop): package chrome extension as tauri resource"
```

---

### Task 7: Add Frontend Setup API Client (TDD)

**Files:**
- Create: `desktop/lib/api/cookie-setup.ts`
- Create: `desktop/lib/api/cookie-setup.test.ts`

**Step 1: Add failing API invoke payload tests**

In `cookie-setup.test.ts`:
```ts
it("calls get_cookie_gateway_status without payload", async () => {
  await getCookieGatewayStatus();
  expect(invokeMock).toHaveBeenCalledWith("get_cookie_gateway_status");
});

it("calls skip_cookie_setup with camelCase reason field", async () => {
  await skipCookieSetup("later");
  expect(invokeMock).toHaveBeenCalledWith("skip_cookie_setup", { reason: "later" });
});
```

**Step 2: Run test and verify fail**

Run:
```bash
pnpm --dir desktop test -- lib/api/cookie-setup.test.ts
```
Expected: FAIL before module exists.

**Step 3: Implement API wrapper module**

In `cookie-setup.ts`:
```ts
import { invoke } from "@tauri-apps/api/core";

export async function getCookieGatewayStatus() {
  return invoke("get_cookie_gateway_status");
}

export async function exportCookieExtensionBundle() {
  return invoke<string>("export_cookie_extension_bundle");
}

export async function getCookieSetupState() {
  return invoke("get_cookie_setup_state");
}

export async function completeCookieSetup() {
  return invoke("complete_cookie_setup");
}

export async function skipCookieSetup(reason?: string) {
  return invoke("skip_cookie_setup", reason ? { reason } : undefined);
}
```

**Step 4: Re-run API tests**

Run:
```bash
pnpm --dir desktop test -- lib/api/cookie-setup.test.ts
```
Expected: PASS.

**Step 5: Commit API module**

Run:
```bash
git add desktop/lib/api/cookie-setup.ts desktop/lib/api/cookie-setup.test.ts
git commit -m "feat(desktop): add cookie setup frontend api client"
```

---

### Task 8: Build Setup Page UI and Behavior (TDD)

**Files:**
- Create: `desktop/app/setup/page.tsx`
- Create: `desktop/components/features/setup/setup-page.test.tsx`

**Step 1: Add failing setup page tests**

In `setup-page.test.tsx`:
```tsx
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";

it("renders 3 setup steps and status cards", async () => {
  render(<SetupPage />);
  expect(screen.getByText("初始化设置")).toBeInTheDocument();
  expect(screen.getByText("步骤 1")).toBeInTheDocument();
  expect(screen.getByText("步骤 2")).toBeInTheDocument();
  expect(screen.getByText("步骤 3")).toBeInTheDocument();
});

it("calls export on click", async () => {
  render(<SetupPage />);
  fireEvent.click(screen.getByRole("button", { name: /下载扩展/i }));
  await waitFor(() => {
    expect(invokeMock).toHaveBeenCalledWith("export_cookie_extension_bundle");
  });
});

it("auto redirects when extension connected and setup completed", async () => {
  invokeMock
    .mockResolvedValueOnce({
      gateway_running: true,
      extension_connected: true,
      setup_phase: "completed",
    });
  render(<SetupPage />);
  await waitFor(() => {
    expect(replaceMock).toHaveBeenCalledWith("/");
  });
});
```

**Step 2: Run tests and confirm fail**

Run:
```bash
pnpm --dir desktop test -- components/features/setup/setup-page.test.tsx
```
Expected: FAIL before component implementation.

**Step 3: Implement setup page**

Implement in `desktop/app/setup/page.tsx`:
1. Poll `getCookieGatewayStatus` with in-flight guard.
2. Show gateway/extension statuses.
3. `下载扩展` triggers `exportCookieExtensionBundle`.
4. `跳过` triggers `skipCookieSetup`.
5. On connected + completed state, redirect to `/`.

Minimal polling logic:
```tsx
const inFlight = useRef(false);
useEffect(() => {
  const check = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      const status = await getCookieGatewayStatus();
      setStatus(status);
      if (status.extension_connected && status.setup_phase === "completed") {
        router.replace("/");
      }
    } finally {
      inFlight.current = false;
    }
  };
  const t = setInterval(() => void check(), 2000);
  void check();
  return () => clearInterval(t);
}, []);
```

**Step 4: Re-run setup page tests**

Run:
```bash
pnpm --dir desktop test -- components/features/setup/setup-page.test.tsx
```
Expected: PASS.

**Step 5: Commit setup page**

Run:
```bash
git add desktop/app/setup/page.tsx desktop/components/features/setup/setup-page.test.tsx
git commit -m "feat(desktop): add cookie setup onboarding page"
```

---

### Task 9: Add Route Guard Component and Integrate in AppLayout (TDD)

**Files:**
- Create: `desktop/components/features/setup/cookie-setup-gate.tsx`
- Create: `desktop/components/features/setup/cookie-setup-gate.test.tsx`
- Modify: `desktop/components/layouts/app-layout.tsx`

**Step 1: Add failing guard behavior tests**

In `cookie-setup-gate.test.tsx`:
```tsx
import { render, waitFor } from "@testing-library/react";
import { vi } from "vitest";

it("redirects to /setup when setup required", async () => {
  pathnameMock.mockReturnValue("/");
  invokeMock.mockResolvedValue({
    setup_phase: "waiting_extension",
    extension_connected: false,
    gateway_running: true,
  });
  render(<CookieSetupGate><div>ok</div></CookieSetupGate>);
  await waitFor(() => expect(replaceMock).toHaveBeenCalledWith("/setup"));
});

it("does not run in non-tauri runtime", async () => {
  isTauriMock.mockReturnValue(false);
  render(<CookieSetupGate><div>ok</div></CookieSetupGate>);
  expect(invokeMock).not.toHaveBeenCalled();
});

it("redirects away from /setup after completion", async () => {
  pathnameMock.mockReturnValue("/setup");
  invokeMock.mockResolvedValue({
    setup_phase: "completed",
    extension_connected: true,
    gateway_running: true,
  });
  render(<CookieSetupGate><div>ok</div></CookieSetupGate>);
  await waitFor(() => expect(replaceMock).toHaveBeenCalledWith("/"));
});
```

**Step 2: Run tests and verify fail**

Run:
```bash
pnpm --dir desktop test -- components/features/setup/cookie-setup-gate.test.tsx
```
Expected: FAIL before component exists.

**Step 3: Implement guard and layout wiring**

In `cookie-setup-gate.tsx`:
```tsx
import { invoke, isTauri } from "@tauri-apps/api/core";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useRef } from "react";

const needsSetup =
  status.setup_phase !== "completed" &&
  status.setup_phase !== "skipped" &&
  !status.extension_connected;
```

In `app-layout.tsx`, wrap content:
```tsx
<CookieSetupGate>{children}</CookieSetupGate>
```

Do not modify `desktop/app/layout.tsx` to client component.

**Step 4: Re-run guard + smoke tests**

Run:
```bash
pnpm --dir desktop test -- components/features/setup/cookie-setup-gate.test.tsx components/layouts/theme-toggle.test.tsx
```
Expected: PASS.

**Step 5: Commit guard integration**

Run:
```bash
git add desktop/components/features/setup/cookie-setup-gate.tsx desktop/components/features/setup/cookie-setup-gate.test.tsx desktop/components/layouts/app-layout.tsx
git commit -m "feat(desktop): gate app routes with cookie setup status"
```

---

### Task 10: Security Hardening for Extension Command Surface (TDD)

**Files:**
- Modify: `cookie-gateway/chrome-extension/background.js`
- Create: `cookie-gateway/chrome-extension/background-security.test.js`

**Step 1: Add failing behavior test**

Create `background-security.test.js`:
```js
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

test("OPEN_URL is disabled by default", () => {
  const code = readFileSync(new URL("./background.js", import.meta.url), "utf8");
  assert.match(code, /const ALLOW_OPEN_URL = false/);
});
```

Run:
```bash
node --test cookie-gateway/chrome-extension/background-security.test.js
```
Expected: FAIL before feature flag exists.

**Step 2: Gate `OPEN_URL` behind explicit feature flag**

In `background.js`:
```js
const ALLOW_OPEN_URL = false;
// in handleCommand switch:
case "OPEN_URL":
  if (!ALLOW_OPEN_URL) throw new Error("OPEN_URL is disabled");
  result = await runOpenUrl(command);
  break;
```

**Step 3: Re-run smoke assertion**

Run:
```bash
node --test cookie-gateway/chrome-extension/background-security.test.js
```
Expected: PASS.

**Step 4: Re-run cookie-gateway tests**

Run:
```bash
cargo test -p cookie-gateway -- --nocapture
```
Expected: PASS (no regressions in Rust side).

**Step 5: Commit hardening**

Run:
```bash
git add cookie-gateway/chrome-extension/background.js cookie-gateway/chrome-extension/background-security.test.js
git commit -m "security(extension): disable OPEN_URL by default"
```

---

### Task 11: End-to-End Verification + Docs Sync

**Files:**
- Modify: `docs/plans/2026-03-04-cookie-gateway-extension-integration-design.md`
- Modify: `docs/plans/2026-03-04-cookie-gateway-extension-integration-implementation-plan.md`

**Step 1: Run full verification commands**

Run:
```bash
cargo test -p cookie-gateway -- --nocapture
cargo test -p desktop -- --nocapture
pnpm --dir desktop test
cargo build -p desktop
```
Expected: all pass.

**Step 2: Run manual acceptance flow**

Checklist:
1. Fresh app data launches to `/setup`
2. Gateway starts and `/health` reachable
3. Exported extension zip is valid
4. Install extension by unzip + Load unpacked
5. Status turns connected and flow can complete
6. Restart skips setup when completed
7. Skip path remains distinct from completed

Expected: all pass.

**Step 3: Synchronize docs with shipped behavior**

Update both docs to reflect actual command names, files, and test commands after implementation.

**Step 4: Final commit**

Run:
```bash
git add docs/plans/2026-03-04-cookie-gateway-extension-integration-design.md docs/plans/2026-03-04-cookie-gateway-extension-integration-implementation-plan.md
git commit -m "docs: sync cookie gateway integration design and implementation plan"
```

---

## Final Delivery Checklist

- [ ] Extension packaging script fixed and deterministic
- [ ] `CookieGateway` exposes `is_extension_connected` and `fetch_cookies`
- [ ] Desktop cookie setup state persisted atomically
- [ ] New Tauri setup commands implemented and registered
- [ ] Setup page `/setup` implemented
- [ ] Guard integrated in `AppLayout` without breaking root layout
- [ ] Security hardening (`OPEN_URL` off by default) landed
- [ ] Rust and frontend tests pass
- [ ] Manual checklist validated
