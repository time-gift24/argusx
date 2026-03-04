# Cookie Gateway Extension Integration Design

**Date:** 2026-03-04  
**Status:** Revised Draft  
**Author:** Claude Sonnet 4.6 (review-reworked by Codex)

## Review-Driven Revision Summary

This revision resolves the core issues found in the previous draft:

1. Aligns with **Tauri v2** (`@tauri-apps/api/core`, v2 `tauri.conf.json` structure)
2. Aligns with **current repository structure** (`desktop/app`, `desktop/components/layouts/app-layout.tsx`, existing commands)
3. Replaces non-existent APIs with implementable interfaces
4. Defines complete onboarding state machine and persistence strategy
5. Adds executable packaging approach and explicit failure handling
6. Strengthens security model for localhost gateway and extension communication
7. Adds concrete performance constraints and test matrix

---

## 1. Overview

This document defines how to integrate `cookie-gateway` into the desktop app, including:

- Extension packaging into desktop bundle resources
- First-run setup experience (`/setup`)
- Gateway and extension connectivity detection
- Setup completion persistence
- Safe cookie retrieval for agent tooling

Primary objective: deliver a stable first-run flow that is implementable in the current codebase without breaking existing desktop runtime behavior.

---

## 2. Current Baseline (Repository Reality)

### 2.1 Desktop

- Tauri version: **v2** (`desktop/src-tauri/Cargo.toml`)
- Frontend invoke API currently uses `@tauri-apps/api/core`
- Root Next.js layout (`desktop/app/layout.tsx`) is currently server layout with metadata and wraps `AppLayout`
- Existing cookie-related commands in `desktop/src-tauri/src/lib.rs`:
  - `get_cookie_opt_in`
  - `set_cookie_opt_in`
  - `open_extension_folder`

### 2.2 Cookie Gateway

- Gateway listens on `127.0.0.1:3456`
- Available routes:
  - `GET /health`
  - `GET /ws` (and aliases)
  - `POST /api/cookies/fetch`
  - `GET|POST /api/cookies`
- Extension command bus exists and already supports request/response flow

### 2.3 Chrome Extension

- Manifest v3 service worker model
- WebSocket client connects to `ws://localhost:3456`
- Supports `GET_COOKIES` and `OPEN_URL` actions

---

## 3. Scope and Non-Goals

### 3.1 Scope

- Desktop setup page and guard
- Tauri commands for setup/status/download
- Extension package build/copy integration
- Setup persistence model
- Cookie fetch path from desktop runtime to extension

### 3.2 Non-Goals (Phase 1)

- Browser auto-detection beyond Chrome
- Auto-install extension into browser profile
- Remote extension update system
- Full diagnostics UI (only minimal troubleshooting copy in setup page)

---

## 4. Architecture

### 4.1 Components

```text
Desktop (Tauri v2 + Next.js)
├── Setup UI (/setup)
├── Route Guard (client component inside AppLayout)
├── Tauri Commands
│   ├── get_cookie_gateway_status
│   ├── export_cookie_extension_bundle
│   ├── get_cookie_setup_state
│   ├── complete_cookie_setup
│   ├── skip_cookie_setup
│   ├── get_cookie_opt_in          (existing)
│   ├── set_cookie_opt_in          (existing)
│   └── open_extension_folder      (existing)
└── AppState
    ├── Arc<CookieGateway>
    ├── Arc<RwLock<CookieSetupState>>
    └── reqwest::Client (reused for health checks)

Cookie Gateway (localhost:3456)
├── /health
├── /ws
├── /api/cookies
└── /api/cookies/fetch

Chrome Extension
├── background.js (WebSocket + command executor)
└── manifest.json
```

### 4.2 Key Interactions

1. Desktop boot starts gateway in background task.
2. Route guard reads setup state + live gateway status.
3. If setup required, redirect to `/setup`.
4. User exports extension bundle, opens extension folder, installs extension.
5. Extension connects over WebSocket.
6. Setup page detects connection, marks setup completed, redirects to `/`.

---

## 5. Setup State Machine

```text
NOT_STARTED
  ├─(user enters setup)────────────> WAITING_EXTENSION
  ├─(user clicks skip)─────────────> SKIPPED
  └─(extension connected + confirm)-> COMPLETED

WAITING_EXTENSION
  ├─(extension connected)──────────> CONNECTED
  ├─(user clicks skip)─────────────> SKIPPED
  └─(app restart)──────────────────> WAITING_EXTENSION

CONNECTED
  └─(complete setup)───────────────> COMPLETED

SKIPPED
  ├─(user opens setup manually)────> WAITING_EXTENSION
  └─(settings reset)───────────────> NOT_STARTED

COMPLETED
  └─(settings reset)───────────────> NOT_STARTED
```

**Important:** `SKIPPED` and `COMPLETED` are different states.  
Do not use a single `setup_completed: bool` for both.

---

## 6. Data Model and Persistence

### 6.1 Rust Data Structures

**File:** `desktop/src-tauri/src/lib.rs` (or extracted `cookie_setup.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CookieSetupPhase {
    NotStarted,
    WaitingExtension,
    Connected,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CookieSetupState {
    pub phase: CookieSetupPhase,
    pub updated_at: String,
    pub skipped_reason: Option<String>,
    pub version: u32,
}

impl Default for CookieSetupState {
    fn default() -> Self {
        Self {
            phase: CookieSetupPhase::NotStarted,
            updated_at: chrono::Utc::now().to_rfc3339(),
            skipped_reason: None,
            version: 1,
        }
    }
}
```

### 6.2 Storage Location

- Path: `<app_data_dir>/cookie_setup_state.json`
- Write policy:
  - write only when state changes
  - atomic write (`*.tmp` then rename)
  - tolerate corruption by resetting to default and logging warning

---

## 7. Backend API Contract (Tauri Commands)

### 7.1 Command Surface

```rust
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CookieGatewayStatus {
    pub gateway_running: bool,
    pub extension_connected: bool,
    pub setup_phase: CookieSetupPhase,
}

#[tauri::command]
async fn get_cookie_gateway_status(state: tauri::State<'_, AppState>) -> Result<CookieGatewayStatus, String>;

#[tauri::command]
async fn export_cookie_extension_bundle(app: tauri::AppHandle) -> Result<String, String>;

#[tauri::command]
async fn get_cookie_setup_state(state: tauri::State<'_, AppState>) -> Result<CookieSetupState, String>;

#[tauri::command]
async fn complete_cookie_setup(state: tauri::State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn skip_cookie_setup(
    reason: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String>;
```

### 7.2 AppState Extension

```rust
struct AppState {
    // existing fields...
    cookie_gateway: std::sync::Arc<cookie_gateway::CookieGateway>,
    cookie_setup_state: std::sync::Arc<tokio::sync::RwLock<CookieSetupState>>,
    cookie_gateway_health_client: reqwest::Client,
}
```

### 7.3 Gateway Accessors Needed in `cookie-gateway`

`cookie-gateway` currently does not expose extension connection status and fetch API on `CookieGateway` directly.  
Add these methods:

**File:** `cookie-gateway/src/lib.rs`

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

**File:** `cookie-gateway/src/command_bus.rs`

```rust
impl GatewayCommandBus {
    pub async fn is_connected(&self) -> bool {
        self.connection.read().await.is_some()
    }
}
```

### 7.4 Health Check Helper (Client Reuse)

```rust
async fn check_gateway_health(client: &reqwest::Client) -> bool {
    client
        .get("http://127.0.0.1:3456/health")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map(|res| res.status().is_success())
        .unwrap_or(false)
}
```

---

## 8. Frontend Design

### 8.1 New Setup Route

- Create: `desktop/app/setup/page.tsx`
- Use `@tauri-apps/api/core` invoke API
- Poll status with interval + in-flight guard to avoid overlapping requests
- Show explicit troubleshooting actions:
  - export extension bundle
  - open extension folder
  - re-check now
  - skip for now

### 8.2 Startup Guard Placement

Do **not** convert `desktop/app/layout.tsx` to client component.  
Keep root layout server-compatible.

Add guard in client layer:

- Modify: `desktop/components/layouts/app-layout.tsx`
- Add `CookieSetupGate` (new component) around `children`

**File:** `desktop/components/features/setup/cookie-setup-gate.tsx`

```tsx
"use client";

import { invoke, isTauri } from "@tauri-apps/api/core";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useRef } from "react";

export function CookieSetupGate({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const inflight = useRef(false);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;

    const check = async () => {
      if (inflight.current) return;
      inflight.current = true;
      try {
        const status = await invoke<{ setup_phase: string; extension_connected: boolean }>(
          "get_cookie_gateway_status",
        );
        if (cancelled) return;

        const needsSetup =
          status.setup_phase !== "completed" &&
          status.setup_phase !== "skipped" &&
          !status.extension_connected;

        if (needsSetup && pathname !== "/setup") router.replace("/setup");
        if (!needsSetup && pathname === "/setup") router.replace("/");
      } finally {
        inflight.current = false;
      }
    };

    void check();
    const timer = setInterval(() => void check(), 3000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [pathname, router]);

  return <>{children}</>;
}
```

---

## 9. Extension Packaging and Distribution

### 9.1 Build Pipeline

1. Build extension archive during Tauri build
2. Copy `chrome-extension.zip` into `desktop/src-tauri/resources/`
3. Bundle as app resource
4. On setup page, export to user-accessible directory

### 9.2 `tauri.conf.json` (v2-aligned)

**File:** `desktop/src-tauri/tauri.conf.json`

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "build": {
    "beforeDevCommand": "pnpm run dev",
    "beforeBuildCommand": "pnpm run build",
    "devUrl": "http://localhost:3000",
    "frontendDist": "../out"
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "resources": [
      "resources/chrome-extension.zip"
    ]
  },
  "app": {
    "security": {
      "csp": "default-src 'self'; connect-src 'self' ws://127.0.0.1:3456 http://127.0.0.1:3456;"
    }
  }
}
```

### 9.3 `build.rs` (executable form)

**File:** `desktop/src-tauri/build.rs`

```rust
use std::path::PathBuf;
use std::process::Command;

fn main() {
    tauri_build::build();
    println!("cargo:rerun-if-changed=../../cookie-gateway/chrome-extension");

    let extension_dir = PathBuf::from("../../cookie-gateway/chrome-extension");
    let script = extension_dir.join("build-extension.sh");

    let status = Command::new("bash")
        .arg(script)
        .current_dir(&extension_dir)
        .status()
        .expect("failed to run extension build script");

    if !status.success() {
        panic!("extension build script failed");
    }

    let from = extension_dir.join("dist/chrome-extension.zip");
    let to_dir = PathBuf::from("resources");
    let to = to_dir.join("chrome-extension.zip");

    std::fs::create_dir_all(&to_dir).expect("failed to create resources dir");
    std::fs::copy(&from, &to).expect("failed to copy extension zip");
}
```

### 9.4 Extension Packaging Script

**File:** `cookie-gateway/chrome-extension/build-extension.sh`

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

---

## 10. Error Handling

### 10.1 Error Model

- Backend returns structured stringified error JSON for UI mapping
- UI maps error code to localized user message
- Fatal errors do not crash setup page; setup stays interactive

### 10.2 Error Categories

| Code | Source | User Action |
|---|---|---|
| `GATEWAY_NOT_RUNNING` | Health check failed | Retry, restart app |
| `EXTENSION_NOT_CONNECTED` | No ws client | Open extension page and reload extension |
| `EXTENSION_EXPORT_FAILED` | File export failed | Choose another directory, check permissions |
| `STATE_PERSIST_FAILED` | JSON write failed | Retry, check disk space |
| `COOKIE_FETCH_FAILED` | Gateway tool path | Retry, inspect domain whitelist / opt-in |

### 10.3 Boundary Cases to Handle Explicitly

1. Port `3456` already in use
2. Extension installed but service worker sleeping (temporary disconnect)
3. Download/export directory unavailable
4. Corrupted setup state file
5. App starts without Tauri context (web dev mode fallback)

---

## 11. Security Design

### 11.1 Localhost Boundary

- Gateway binds `127.0.0.1` only
- Frontend CSP only allows localhost gateway endpoints
- No external endpoint required for setup flow

### 11.2 Extension Command Surface Hardening

Phase 1 requirement:

- Keep `GET_COOKIES` enabled
- Gate `OPEN_URL` behind explicit feature flag (default off)
- Reject unknown action types

### 11.3 Handshake and Authentication (Required for production hardening)

Add lightweight handshake token:

1. Desktop generates random token at boot
2. Gateway sends challenge on connection
3. Extension must echo signed token in `CLIENT_HELLO`
4. Unverified clients are disconnected and ignored

This mitigates unauthorized local process command injection.

### 11.4 Sensitive Data Handling

- Never log raw cookie values
- UI shows counts/status only
- Cookie payload kept in memory; no disk persistence for cookie contents

---

## 12. Performance Design

### 12.1 Polling Strategy

- Setup page polling:
  - first 60s: every 2s
  - after 60s: every 5s
- Use in-flight guard to avoid stacked requests

### 12.2 Backend Efficiency

- Reuse one `reqwest::Client` in `AppState`
- Write setup state only on actual transition
- Use in-memory cookie cache with configurable `refresh_after_ms` (default 5 min)

### 12.3 Target Limits

- Setup status check P95 latency < 100ms (local)
- CPU overhead from polling < 1% while setup page is open
- No unbounded pending request growth on repeated reconnect failures

---

## 13. User Experience and Onboarding

### 13.1 Setup Page Steps

1. Export extension package
2. Open extension folder and installation instructions
3. Verify live connection
4. Confirm completion

### 13.2 UX Requirements

- Must show current state (`gateway_running`, `extension_connected`, `setup_phase`)
- Must expose retry button
- Must provide skip action with explicit warning
- Must auto-redirect only after state becomes `completed`

### 13.3 Copy Improvements

- Replace ambiguous "拖拽 zip 即可安装" with browser-version-safe instructions:
  - unzip package
  - open `chrome://extensions`
  - enable developer mode
  - click "Load unpacked"
  - choose extracted directory

---

## 14. Testing Strategy

### 14.1 Rust Unit and Integration Tests

1. `CookieSetupState` persistence read/write and corruption recovery
2. Gateway status command with mocked health client
3. `skip` vs `complete` transition correctness
4. `is_extension_connected` behavior with/without active websocket client

Run:

```bash
cargo test -p cookie-gateway
cargo test -p desktop cookie
```

### 14.2 Frontend Tests (Vitest + RTL)

1. Setup page renders step-by-step guidance
2. Error toast appears for command failure
3. Auto-redirect when status switches to connected/completed
4. Route guard redirects correctly

Run:

```bash
pnpm --dir desktop test
```

### 14.3 Manual Acceptance Checklist

- [ ] Fresh install enters `/setup`
- [ ] Gateway auto-starts and `/health` returns 200
- [ ] Extension package exports successfully
- [ ] Extension connection badge turns green after install
- [ ] Skip does not mark setup as completed
- [ ] Restart respects `completed` and skips setup
- [ ] Cookie fetch from chat tool works for whitelisted domain

---

## 15. Technical Debt and Follow-ups

1. Move setup persistence from JSON file to SQLite table (unify app state storage)
2. Replace polling with push events from backend (`tauri::Emitter`)
3. Add diagnostics page for connection troubleshooting
4. Add browser matrix support (Edge, Firefox)
5. Add extension update mechanism and signature validation

---

## 16. Migration and Rollout

### Phase 1

- Add backend command surface and setup state storage
- Add setup page and route guard
- Package extension into resources

### Phase 2

- Integrate cookie tool usage in chat workflows
- Add security handshake token

### Phase 3

- Observability and diagnostics
- Multi-browser support

---

## 17. Success Metrics

- Setup completion rate >= 90%
- Median setup time <= 2 minutes
- Extension connection success rate >= 95%
- Setup-related fatal errors per 1k sessions < 5
- Cookie fetch success rate (whitelisted domains) >= 99%

---

## 18. Open Questions

1. Should `OPEN_URL` remain enabled in extension for Phase 1, or be disabled by default?
2. Is JSON state storage acceptable for Phase 1, or should we directly use SQLite now?
3. Should we expose a settings entry to re-run setup after skip/completion?

---

## 19. References

- [Cookie Gateway Core](../../cookie-gateway/src/lib.rs)
- [Gateway HTTP Routes](../../cookie-gateway/src/gateway.rs)
- [Gateway Command Bus](../../cookie-gateway/src/command_bus.rs)
- [Desktop Tauri Commands](../../desktop/src-tauri/src/lib.rs)
- [Desktop App Layout](../../desktop/components/layouts/app-layout.tsx)
- [Tauri v2 Commands](https://tauri.app/develop/calling-rust/)
- [Next.js App Router](https://nextjs.org/docs/app)
