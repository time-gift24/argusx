# Cookie Gateway Extension Integration Design

**Date:** 2026-03-04
**Status:** Draft
**Author:** Claude Sonnet 4.6

## Overview

This document describes the design for integrating Cookie Gateway into the ArgusX desktop application, including Chrome extension packaging, distribution, and user onboarding flow.

## Problem Statement

1. Cookie Gateway is implemented but not integrated into the desktop application
2. Chrome extension needs to be packaged and distributed to users
3. Users need guidance to install the extension on first launch
4. Connection status needs to be monitored and displayed in real-time
5. Setup completion should be tracked to avoid showing the setup page on subsequent launches

## Design Goals

- **Seamless Integration:** Cookie Gateway should be a core capability for SRE agents
- **User-Friendly Onboarding:** Clear step-by-step guide for extension installation
- **Real-Time Feedback:** Visual indication of connection status
- **Automatic Setup:** Detect successful connection and skip setup on future launches
- **Error Resilience:** Graceful handling of missing extension or connection failures

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Desktop App (Tauri)                       │
│                                                              │
│  ┌──────────────────┐         ┌─────────────────────────┐  │
│  │  Setup Page      │         │  Main Dashboard         │  │
│  │  /app/setup      │         │  /app & /app/chat       │  │
│  │                  │         │                         │  │
│  │  - Extension DL  │◄───────►│  - Agent Chat           │  │
│  │  - Install Guide │         │  - Use Cookie Tool      │  │
│  │  - Conn Status   │         │                         │  │
│  └────────┬─────────┘         └───────────┬─────────────┘  │
│           │                               │                  │
│           │ invoke()                      │ invoke()         │
│           ▼                               ▼                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │      Tauri Commands (src-tauri/src/lib.rs)           │   │
│  │                                                       │   │
│  │  - get_extension_status()                            │   │
│  │  - download_extension() -> .crx file path            │   │
│  │  - check_gateway_connection() -> bool                │   │
│  │  - fetch_cookies(domain) -> Vec<CookieData>          │   │
│  │  - get_init_state() -> InitState                     │   │
│  │  - set_init_completed()                              │   │
│  └────────┬──────────────────────────────────────────────┘   │
└───────────┼──────────────────────────────────────────────────┘
            │
            │ Rust API calls (Arc<CookieGateway>)
            ▼
┌───────────────────────────────────────────────────────────┐
│         Cookie Gateway (localhost:3456)                    │
│                                                            │
│  ┌──────────────┐         ┌──────────────────────┐        │
│  │ HTTP Server  │         │  WebSocket Server    │        │
│  │  /health     │         │  /ws                 │        │
│  │  /api/cookies│◄────────┤                      │        │
│  │  /api/fetch  │         │  Chrome Extension    │        │
│  └──────────────┘         │  Connection          │        │
│                           └──────────────────────┘        │
└───────────────────────────────────────────────────────────┘
            ▲
            │ WebSocket (ws://localhost:3456)
            │
┌───────────────────────────────────────────────────────────┐
│         Chrome Extension (cookie-gateway-client)           │
│                                                            │
│  - background.js (WebSocket client)                        │
│  - Responds to GET_COOKIES commands                        │
│  - Auto-reconnect on disconnect                            │
└───────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Gateway Lifecycle:** Cookie Gateway starts automatically when Tauri app launches using `tokio::spawn`
2. **Extension Distribution:** Extension packaged as Tauri resource, copied to user directory on download
3. **State Persistence:** Init state stored in `~/.config/argusx/init_state.json`
4. **Connection Detection:** Setup page polls every 2 seconds via `check_gateway_connection()`

## Implementation Details

### 1. Tauri Commands

**File:** `desktop/src-tauri/src/lib.rs`

#### Data Structures

```rust
use tauri::State;
use std::sync::Arc;
use cookie_gateway::{CookieGateway, CookieData};

pub struct AppState {
    cookie_gateway: Arc<CookieGateway>,
    init_state: Arc<Mutex<InitState>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct InitState {
    pub extension_installed: bool,
    pub setup_completed: bool,
    pub last_connection_check: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ExtensionStatus {
    pub gateway_running: bool,
    pub extension_connected: bool,
    pub extension_path: Option<String>,
}

#[derive(serde::Serialize)]
pub struct CookieFetchResult {
    pub domain: String,
    pub count: usize,
    pub source: String, // "cache" | "refresh"
    pub cookies: Vec<CookieData>,
}
```

#### Command Functions

```rust
// 1. Check extension and gateway status
#[tauri::command]
async fn get_extension_status(
    state: State<'_, Arc<AppState>>,
) -> Result<ExtensionStatus, String> {
    let gateway_running = check_gateway_health().await?;
    let extension_connected = state.cookie_gateway.has_connected_extension().await;
    let extension_path = get_extension_resource_path()?;

    Ok(ExtensionStatus {
        gateway_running,
        extension_connected,
        extension_path: Some(extension_path),
    })
}

// 2. Download/export extension file
#[tauri::command]
async fn download_extension(
    app: tauri::AppHandle,
) -> Result<String, String> {
    let resource_path = get_extension_resource_path()?;
    let download_dir = app.path_resolver()
        .resolve_path(ResourceType::Download)
        .ok_or("Failed to resolve download directory")?;

    let dest_path = download_dir.join("argusx-cookie-extension.zip");

    std::fs::copy(&resource_path, &dest_path)
        .map_err(|e| format!("Failed to copy extension: {}", e))?;

    Ok(dest_path.to_string_lossy().to_string())
}

// 3. Check gateway connection status
#[tauri::command]
async fn check_gateway_connection(
    state: State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    let health = check_gateway_health().await?;

    if health {
        let mut init_state = state.init_state.lock().await;
        init_state.last_connection_check = Some(chrono::Utc::now().to_rfc3339());
        save_init_state(&init_state)?;
    }

    Ok(health && state.cookie_gateway.has_connected_extension().await)
}

// 4. Fetch cookies for specified domain
#[tauri::command]
async fn fetch_cookies(
    domain: String,
    refresh_after_ms: Option<u64>,
    state: State<'_, Arc<AppState>>,
) -> Result<CookieFetchResult, String> {
    let refresh_after = std::time::Duration::from_millis(
        refresh_after_ms.unwrap_or(300_000) // Default 5 minutes
    );

    let result = state.cookie_gateway
        .fetch_cookies(&domain, refresh_after)
        .await
        .map_err(|e| format!("Failed to fetch cookies: {}", e))?;

    Ok(CookieFetchResult {
        domain: result.domain,
        count: result.count,
        source: match result.source {
            cookie_gateway::CookieFetchSource::Cache => "cache".to_string(),
            cookie_gateway::CookieFetchSource::Refresh => "refresh".to_string(),
        },
        cookies: result.cookies,
    })
}

// 5. Get initialization state
#[tauri::command]
async fn get_init_state(
    state: State<'_, Arc<AppState>>,
) -> Result<InitState, String> {
    let init_state = state.init_state.lock().await;
    Ok(init_state.clone())
}

// 6. Mark initialization as completed
#[tauri::command]
async fn set_init_completed(
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut init_state = state.init_state.lock().await;
    init_state.setup_completed = true;
    save_init_state(&init_state)?;
    Ok(())
}
```

#### Helper Functions

```rust
async fn check_gateway_health() -> Result<bool, String> {
    use reqwest::Client;
    let client = Client::new();

    match client.get("http://localhost:3456/health").send().await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

fn get_extension_resource_path() -> Result<String, String> {
    Ok("resources/chrome-extension.zip".to_string())
}

fn save_init_state(state: &InitState) -> Result<(), String> {
    // Save to app_data_dir/init_state.json
    // Implementation omitted for brevity
    Ok(())
}
```

#### App Initialization

```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Start cookie gateway
            let gateway = CookieGateway::new(CookieStore::new());
            let gateway_clone = gateway.clone();

            tauri::async_runtime::spawn(async move {
                gateway_clone.start().await.expect("Failed to start cookie gateway");
            });

            // Initialize app state
            let init_state = load_or_create_init_state()?;
            let app_state = Arc::new(AppState {
                cookie_gateway: gateway,
                init_state: Arc::new(Mutex::new(init_state)),
            });

            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_extension_status,
            download_extension,
            check_gateway_connection,
            fetch_cookies,
            get_init_state,
            set_init_completed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2. Frontend Setup Page

**File:** `desktop/app/setup/page.tsx`

```typescript
"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { invoke } from "@tauri-apps/api/tauri";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  CheckCircle2,
  XCircle,
  Download,
  ExternalLink,
  Loader2,
  Chrome,
} from "lucide-react";
import { useToast } from "@/components/ui/use-toast";

interface ExtensionStatus {
  gateway_running: boolean;
  extension_connected: boolean;
  extension_path: string | null;
}

interface InitState {
  extension_installed: boolean;
  setup_completed: boolean;
  last_connection_check: string | null;
}

export default function SetupPage() {
  const router = useRouter();
  const { toast } = useToast();
  const [status, setStatus] = useState<ExtensionStatus | null>(null);
  const [downloading, setDownloading] = useState(false);

  // Check connection status every 2 seconds
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const status = await invoke<ExtensionStatus>("get_extension_status");
        setStatus(status);

        // If extension connected and setup not completed, mark as complete
        if (status.extension_connected) {
          const initState = await invoke<InitState>("get_init_state");
          if (!initState.setup_completed) {
            await invoke("set_init_completed");
            toast({
              title: "设置完成",
              description: "Chrome 扩展已成功连接，正在跳转...",
            });
            setTimeout(() => router.push("/"), 2000);
          }
        }
      } catch (error) {
        console.error("Failed to check status:", error);
      }
    };

    checkStatus();
    const interval = setInterval(checkStatus, 2000);
    return () => clearInterval(interval);
  }, [router, toast]);

  const handleDownloadExtension = async () => {
    setDownloading(true);
    try {
      const path = await invoke<string>("download_extension");
      toast({
        title: "扩展已下载",
        description: `文件保存在: ${path}`,
      });
    } catch (error) {
      toast({
        title: "下载失败",
        description: String(error),
        variant: "destructive",
      });
    } finally {
      setDownloading(false);
    }
  };

  const handleOpenExtensions = () => {
    invoke("open_chrome_extensions").catch(console.error);
  };

  const handleSkip = async () => {
    try {
      await invoke("set_init_completed");
      router.push("/");
    } catch (error) {
      console.error("Failed to skip setup:", error);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4">
      <Card className="w-full max-w-2xl">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-2xl">初始化设置</CardTitle>
              <CardDescription className="mt-1">
                安装 Chrome 扩展以启用 Cookie Gateway 功能
              </CardDescription>
            </div>
            <Chrome className="h-8 w-8 text-muted-foreground" />
          </div>
        </CardHeader>

        <CardContent className="space-y-6">
          {/* Step 1: Download Extension */}
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Badge variant="outline">步骤 1</Badge>
              <span className="font-semibold">下载 Chrome 扩展</span>
            </div>
            <Button
              onClick={handleDownloadExtension}
              disabled={downloading}
              className="w-full"
            >
              {downloading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  下载中...
                </>
              ) : (
                <>
                  <Download className="mr-2 h-4 w-4" />
                  下载扩展文件 (.zip)
                </>
              )}
            </Button>
          </div>

          <Separator />

          {/* Step 2: Install Extension */}
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Badge variant="outline">步骤 2</Badge>
              <span className="font-semibold">安装扩展</span>
            </div>
            <ol className="list-decimal list-inside space-y-2 text-sm text-muted-foreground">
              <li>打开 Chrome 浏览器</li>
              <li>
                在地址栏输入{" "}
                <code className="bg-muted px-1 rounded">chrome://extensions</code>
              </li>
              <li>开启右上角的"开发者模式"</li>
              <li>将下载的 .zip 文件拖拽到页面中</li>
              <li>点击"添加扩展程序"确认安装</li>
            </ol>
            <Button variant="outline" onClick={handleOpenExtensions}>
              <ExternalLink className="mr-2 h-4 w-4" />
              打开 Chrome 扩展页面
            </Button>
          </div>

          <Separator />

          {/* Step 3: Verify Connection */}
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Badge variant="outline">步骤 3</Badge>
              <span className="font-semibold">验证连接状态</span>
            </div>

            <div className="space-y-2">
              {/* Gateway Status */}
              <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
                <div className="flex items-center gap-2">
                  {status?.gateway_running ? (
                    <CheckCircle2 className="h-5 w-5 text-green-500" />
                  ) : (
                    <XCircle className="h-5 w-5 text-red-500" />
                  )}
                  <span className="text-sm">Cookie Gateway 服务</span>
                </div>
                <Badge variant={status?.gateway_running ? "default" : "secondary"}>
                  {status?.gateway_running ? "运行中" : "未启动"}
                </Badge>
              </div>

              {/* Extension Status */}
              <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
                <div className="flex items-center gap-2">
                  {status?.extension_connected ? (
                    <CheckCircle2 className="h-5 w-5 text-green-500" />
                  ) : (
                    <Loader2 className="h-5 w-5 animate-spin text-yellow-500" />
                  )}
                  <span className="text-sm">Chrome 扩展连接</span>
                </div>
                <Badge variant={status?.extension_connected ? "default" : "secondary"}>
                  {status?.extension_connected ? "已连接" : "等待连接"}
                </Badge>
              </div>
            </div>

            {status?.extension_connected && (
              <div className="p-3 bg-green-500/10 border border-green-500/20 rounded-lg">
                <p className="text-sm text-green-700 flex items-center gap-2">
                  <CheckCircle2 className="h-4 w-4" />
                  扩展连接成功！正在跳转到主页...
                </p>
              </div>
            )}
          </div>

          <Separator />

          {/* Skip Button */}
          <div className="flex justify-end">
            <Button variant="ghost" onClick={handleSkip}>
              跳过设置
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
```

### 3. Startup Flow

**File:** `desktop/app/layout.tsx`

```typescript
"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { invoke } from "@tauri-apps/api/tauri";
import { Loader2 } from "lucide-react";

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const [checking, setChecking] = useState(true);

  useEffect(() => {
    const checkInitState = async () => {
      try {
        const state = await invoke<{
          extension_installed: boolean;
          setup_completed: boolean;
          last_connection_check: string | null;
        }>("get_init_state");

        // If setup not completed and extension not connected, redirect to setup
        if (!state.setup_completed) {
          const status = await invoke<{
            gateway_running: boolean;
            extension_connected: boolean;
          }>("get_extension_status");

          if (!status.extension_connected) {
            router.push("/setup");
          }
        }
      } catch (error) {
        console.error("Failed to check init state:", error);
      } finally {
        setChecking(false);
      }
    };

    checkInitState();
  }, [router]);

  if (checking) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin mx-auto mb-4" />
          <p className="text-muted-foreground">加载中...</p>
        </div>
      </div>
    );
  }

  return (
    // ... original layout content
  );
}
```

### 4. Extension Packaging

**File:** `desktop/src-tauri/tauri.conf.json`

```json
{
  "build": {
    "beforeBuildCommand": "pnpm build",
    "beforeDevCommand": "pnpm dev",
    "devPath": "http://localhost:3000",
    "distDir": "../out"
  },
  "tauri": {
    "bundle": {
      "resources": [
        "resources/*"
      ]
    },
    "security": {
      "csp": "default-src 'self'; connect-src 'self' ws://localhost:3456 http://localhost:3456"
    }
  }
}
```

**File:** `desktop/src-tauri/build.rs`

```rust
use std::process::Command;
use std::path::Path;

fn main() {
    // Build Chrome extension before Tauri app
    println!("cargo:rerun-if-changed=../cookie-gateway/chrome-extension");

    // Run extension packaging script
    let output = Command::new("../cookie-gateway/chrome-extension/build-extension.sh")
        .current_dir("../cookie-gateway/chrome-extension")
        .output()
        .expect("Failed to build Chrome extension");

    if !output.status.success() {
        panic!("Chrome extension build failed: {:?}", output.stderr);
    }

    // Copy extension to resources directory
    let from = Path::new("../cookie-gateway/chrome-extension/dist/chrome-extension.zip");
    let to = Path::new("resources/chrome-extension.zip");
    std::fs::create_dir_all("resources").expect("Failed to create resources dir");
    std::fs::copy(from, to).expect("Failed to copy extension");

    println!("cargo:warning=Chrome extension packaged successfully");
}
```

**File:** `cookie-gateway/chrome-extension/build-extension.sh`

```bash
#!/bin/bash

set -e

echo "Building Chrome extension..."

# Create output directory
mkdir -p dist

# Package extension (simplified: create zip directly)
cd "$(dirname "$0")"
zip -r dist/chrome-extension.zip \
    manifest.json \
    background.js \
    popup.html

echo "Extension packaged: dist/chrome-extension.zip"
echo "Size: $(du -h dist/chrome-extension.zip | cut -f1)"
```

### 5. Error Handling

**File:** `desktop/src-tauri/src/lib.rs`

```rust
#[derive(Debug, serde::Serialize)]
pub enum AppError {
    GatewayNotRunning,
    ExtensionNotConnected,
    CookieFetchFailed(String),
    FileSystemError(String),
    StateError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppError::GatewayNotRunning => write!(f, "Cookie Gateway 服务未运行"),
            AppError::ExtensionNotConnected => write!(f, "Chrome 扩展未连接"),
            AppError::CookieFetchFailed(msg) => write!(f, "获取 Cookie 失败: {}", msg),
            AppError::FileSystemError(msg) => write!(f, "文件操作失败: {}", msg),
            AppError::StateError(msg) => write!(f, "状态管理错误: {}", msg),
        }
    }
}
```

**File:** `desktop/lib/errors.ts`

```typescript
export class AppError extends Error {
  constructor(
    public code: string,
    message: string,
    public userMessage?: string
  ) {
    super(message);
  }
}

export const ERROR_CODES = {
  GATEWAY_NOT_RUNNING: "GATEWAY_NOT_RUNNING",
  EXTENSION_NOT_CONNECTED: "EXTENSION_NOT_CONNECTED",
  COOKIE_FETCH_FAILED: "COOKIE_FETCH_FAILED",
} as const;
```

## Testing Strategy

### Unit Tests (Rust)

```rust
#[tokio::test]
async fn test_get_extension_status() {
    let state = create_test_state().await;
    let status = get_extension_status(State(state)).await.unwrap();

    assert!(status.gateway_running);
    assert!(!status.extension_connected);
}

#[tokio::test]
async fn test_fetch_cookies_without_extension() {
    let state = create_test_state().await;

    let result = fetch_cookies(
        "example.com".to_string(),
        None,
        State(state),
    ).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::ExtensionNotConnected => (),
        _ => panic!("Expected ExtensionNotConnected error"),
    }
}
```

### E2E Tests (TypeScript)

```typescript
test("should redirect to setup page on first launch", async ({ page }) => {
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await expect(page).toHaveURL("/setup");
});

test("should display connection status", async ({ page }) => {
  await page.goto("/setup");

  const gatewayStatus = page.getByText("Cookie Gateway 服务");
  await expect(gatewayStatus).toContainText("运行中");

  const extensionStatus = page.getByText("Chrome 扩展连接");
  await expect(extensionStatus).toContainText("等待连接");
});
```

## Manual Testing Checklist

- [ ] Clear app data and launch, should redirect to /setup
- [ ] Gateway service should auto-start (localhost:3456/health)
- [ ] Download extension button should work
- [ ] Extension should install successfully in Chrome
- [ ] Setup page should show real-time connection status
- [ ] Should auto-redirect to main page on successful connection
- [ ] State should persist (restart should skip setup)
- [ ] Cookie fetching should work in Chat page
- [ ] Error messages should be user-friendly

## Performance Considerations

- **Connection Check Frequency:** 2-second interval to balance responsiveness and resource usage
- **State Caching:** Init state only written when changed
- **WebSocket Reuse:** Gateway uses single WebSocket connection
- **Cookie Caching:** Default 5-minute cache to reduce extension requests

## Security Considerations

- Extension only connects to localhost:3456
- CSP configured to allow WebSocket connections
- No external network requests from extension
- Cookies stored in memory, not persisted to disk

## Future Enhancements

1. **Auto-detect extension installation:** Monitor registry/file system for Chrome extension
2. **Guided tour:** Add interactive tutorial after setup
3. **Multiple browser support:** Support Firefox, Edge extensions
4. **Connection diagnostics:** Add troubleshooting page for connection issues
5. **Extension auto-update:** Implement update mechanism for extension

## Migration Path

1. **Phase 1:** Implement basic setup flow (this design)
2. **Phase 2:** Add cookie usage in agent tools
3. **Phase 3:** Integrate with specific SRE workflows
4. **Phase 4:** Add advanced features (auto-detect, multi-browser)

## Success Metrics

- Setup completion rate > 90%
- Time to complete setup < 2 minutes
- Extension connection success rate > 95%
- Zero data loss on app restart
- User satisfaction score > 4.5/5

## References

- [Cookie Gateway Implementation](../../cookie-gateway/src/lib.rs)
- [Chrome Extension Manifest](../../cookie-gateway/chrome-extension/manifest.json)
- [Tauri Commands Documentation](https://tauri.app/v1/guides/features/command/)
- [Next.js App Router](https://nextjs.org/docs/app)
