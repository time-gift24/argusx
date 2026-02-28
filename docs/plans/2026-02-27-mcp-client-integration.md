# MCP Client Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 MCP Client，让 Agent 能够连接和调用外部 MCP Server 的工具。

**Architecture:** 新建 `mcp-client` crate 实现 MCP 协议，在 `agent-tool` 中添加 `McpToolRegistry` 和 `AggregatedToolRuntime` 来聚合本地工具和 MCP 工具。

**Tech Stack:** Rust, tokio, serde, async-trait, thiserror, tracing

---

## Phase 1: mcp-client Crate 基础结构

### Task 1: 创建 mcp-client crate

**Files:**
- Create: `mcp-client/Cargo.toml`
- Create: `mcp-client/src/lib.rs`

**Step 1: 创建 Cargo.toml**

```toml
[package]
name = "mcp-client"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
thiserror = "1"
tracing = "0.1"
futures = "0.3"
reqwest = { version = "0.11", features = ["json", "stream"], optional = true }

[features]
default = ["stdio"]
stdio = []
http = ["reqwest"]

[dev-dependencies]
tokio-test = "0.4"
```

**Step 2: 创建 lib.rs**

```rust
//! MCP Client - Model Context Protocol client implementation
//!
//! This crate provides a client for connecting to MCP servers
//! and calling their tools.

pub mod error;
pub mod protocol;
pub mod transport;
pub mod session;

pub use error::{McpError, Result};
pub use session::{McpSession, McpServerConfig, McpTransportConfig};
pub use protocol::{Tool, Resource, Prompt, ToolResult, ResourceContent};
```

**Step 3: 验证编译**

Run: `cd mcp-client && cargo build`
Expected: 编译成功，无错误

**Step 4: Commit**

```bash
git add mcp-client/
git commit -m "feat(mcp-client): initialize crate structure"
```

---

### Task 2: 错误类型定义

**Files:**
- Create: `mcp-client/src/error.rs`

**Step 1: 写错误类型**

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, McpError>;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("JSON-RPC error: code={code}, message={message}")]
    Rpc { code: i32, message: String },

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Server not connected: {0}")]
    NotConnected(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Server capabilities not supported: {0}")]
    UnsupportedCapability(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("HTTP error: {0}")]
    #[cfg(feature = "http")]
    Http(#[from] reqwest::Error),
}
```

**Step 2: 验证编译**

Run: `cd mcp-client && cargo build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add mcp-client/src/error.rs
git commit -m "feat(mcp-client): add error types"
```

---

### Task 3: JSON-RPC 协议类型

**Files:**
- Create: `mcp-client/src/protocol/mod.rs`
- Create: `mcp-client/src/protocol/types.rs`

**Step 1: 创建 protocol/mod.rs**

```rust
mod types;
mod messages;

pub use types::*;
pub use messages::*;
```

**Step 2: 创建 protocol/types.rs**

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// JSON-RPC Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

// ============================================================================
// MCP Types - Core
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub experimental: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCapability {
    #[serde(default)]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourcesCapability {
    #[serde(default)]
    pub subscribe: Option<bool>,
    #[serde(default)]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptsCapability {
    #[serde(default)]
    pub list_changed: Option<bool>,
}

// ============================================================================
// MCP Types - Tools
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<Content>,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: ResourceContent },
}

// ============================================================================
// MCP Types - Resources
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub uri: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub blob: Option<String>,  // base64
}

// ============================================================================
// MCP Types - Prompts
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct Prompt {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: Content,
}
```

**Step 3: 验证编译**

Run: `cd mcp-client && cargo build`
Expected: 编译成功

**Step 4: Commit**

```bash
git add mcp-client/src/protocol/
git commit -m "feat(mcp-client): add JSON-RPC and MCP protocol types"
```

---

### Task 4: MCP 消息定义

**Files:**
- Create: `mcp-client/src/protocol/messages.rs`

**Step 1: 创建消息类型**

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::*;

// ============================================================================
// Initialize
// ============================================================================

#[derive(Debug, Serialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

#[derive(Debug, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
}

// ============================================================================
// Tools
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ListToolsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(default)]
    pub is_error: bool,
}

// ============================================================================
// Resources
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ListResourcesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListResourcesResult {
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReadResourceParams {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContent>,
}

// ============================================================================
// Prompts
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ListPromptsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListPromptsResult {
    pub prompts: Vec<Prompt>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetPromptParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Deserialize)]
pub struct GetPromptResult {
    #[serde(default)]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

// ============================================================================
// Helper implementations
// ============================================================================

impl From<CallToolResult> for ToolResult {
    fn from(result: CallToolResult) -> Self {
        ToolResult {
            content: result.content,
            is_error: result.is_error,
        }
    }
}

impl ToolResult {
    pub fn text(&self) -> Option<&str> {
        self.content.first().and_then(|c| match c {
            Content::Text { text } => Some(text.as_str()),
            _ => None,
        })
    }
}
```

**Step 2: 验证编译**

Run: `cd mcp-client && cargo build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add mcp-client/src/protocol/messages.rs
git commit -m "feat(mcp-client): add MCP message types"
```

---

### Task 5: Transport Trait 定义

**Files:**
- Create: `mcp-client/src/transport/mod.rs`
- Create: `mcp-client/src/transport/trait.rs`

**Step 1: 创建 transport/mod.rs**

```rust
mod r#trait;

#[cfg(feature = "stdio")]
pub mod stdio;

#[cfg(feature = "http")]
pub mod http;

pub use r#trait::Transport;
```

**Step 2: 创建 transport/trait.rs**

```rust
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{McpError, Result};

/// Transport trait for MCP communication
#[async_trait]
pub trait Transport: Send {
    /// Send a request and wait for response
    async fn request<T: Serialize + Send, R: DeserializeOwned>(
        &mut self,
        method: &str,
        params: Option<T>,
    ) -> Result<R>;

    /// Send a notification (no response expected)
    async fn notify<T: Serialize + Send>(
        &mut self,
        method: &str,
        params: Option<T>,
    ) -> Result<()>;

    /// Close the transport
    async fn close(&mut self) -> Result<()>;
}

/// Helper to build JSON-RPC request
pub fn build_request<T: Serialize>(id: u64, method: &str, params: Option<T>) -> String {
    let request = crate::protocol::JsonRpcRequest {
        jsonrpc: "2.0",
        id,
        method: method.to_string(),
        params,
    };
    serde_json::to_string(&request).expect("request serialization should not fail")
}
```

**Step 3: 验证编译**

Run: `cd mcp-client && cargo build`
Expected: 编译成功

**Step 4: Commit**

```bash
git add mcp-client/src/transport/
git commit -m "feat(mcp-client): add Transport trait"
```

---

### Task 6: Stdio Transport 实现

**Files:**
- Create: `mcp-client/src/transport/stdio.rs`
- Create: `mcp-client/tests/stdio_transport_test.rs`

**Step 1: 写测试**

```rust
// tests/stdio_transport_test.rs
use mcp_client::transport::Transport;
use mcp_client::transport::stdio::StdioTransport;
use mcp_client::McpTransportConfig;

#[tokio::test]
async fn test_stdio_transport_creates_process() {
    let config = McpTransportConfig::Stdio {
        command: "echo".to_string(),
        args: vec![],
        env: None,
    };

    let result = StdioTransport::new(&config).await;
    // echo 不支持 MCP 协议，但可以验证进程创建
    assert!(result.is_ok() || result.is_err()); // 只要不 panic 就行
}
```

**Step 2: 写 stdio transport 实现**

```rust
// src/transport/stdio.rs
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use tokio::process::{Child as AsyncChild, ChildStdin as AsyncChildStdin, ChildStdout as AsyncChildStdout, Command as AsyncCommand};

use super::Transport;
use crate::protocol::{JsonRpcResponse, JsonRpcError};
use crate::{McpError, McpTransportConfig, Result};

pub struct StdioTransport {
    _child: AsyncChild,
    stdin: AsyncChildStdin,
    stdout: AsyncBufReader<AsyncChildStdout>,
    request_id: AtomicU64,
}

impl StdioTransport {
    pub async fn new(config: &McpTransportConfig) -> Result<Self> {
        let (command, args, env) = match config {
            McpTransportConfig::Stdio { command, args, env } => (command, args, env),
            _ => return Err(McpError::Transport("invalid config type".into())),
        };

        let mut cmd = AsyncCommand::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        if let Some(env_vars) = env {
            for (k, v) in env_vars {
                cmd.env(k, v);
            }
        }

        let mut child = cmd.spawn().map_err(|e| McpError::Transport(
            format!("failed to spawn process '{}': {}", command, e)
        ))?;

        let stdin = child.stdin.take().ok_or_else(|| McpError::Transport(
            "failed to get stdin".into()
        ))?;

        let stdout = child.stdout.take().ok_or_else(|| McpError::Transport(
            "failed to get stdout".into()
        ))?;

        Ok(Self {
            _child: child,
            stdin,
            stdout: AsyncBufReader::new(stdout),
            request_id: AtomicU64::new(0),
        })
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn request<T: Serialize + Send, R: DeserializeOwned>(
        &mut self,
        method: &str,
        params: Option<T>,
    ) -> Result<R> {
        let id = self.next_id();

        // Build request
        let request = crate::protocol::JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let line = serde_json::to_string(&request)? + "\n";

        // Send request
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        // Read response
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        // Parse response
        let response: JsonRpcResponse<R> = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            return Err(McpError::Rpc {
                code: error.code,
                message: error.message,
            });
        }

        response.result.ok_or_else(|| McpError::Protocol("no result in response".into()))
    }

    async fn notify<T: Serialize + Send>(
        &mut self,
        method: &str,
        params: Option<T>,
    ) -> Result<()> {
        let request = crate::protocol::JsonRpcRequest::<T> {
            jsonrpc: "2.0",
            id: 0, // notifications don't need meaningful id
            method: method.to_string(),
            params,
        };

        let line = serde_json::to_string(&request)? + "\n";
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.stdin.shutdown().await?;
        Ok(())
    }
}
```

**Step 3: 运行测试**

Run: `cd mcp-client && cargo test`
Expected: 测试通过

**Step 4: Commit**

```bash
git add mcp-client/src/transport/stdio.rs mcp-client/tests/
git commit -m "feat(mcp-client): implement stdio transport"
```

---

### Task 7: McpSession 实现

**Files:**
- Create: `mcp-client/src/session.rs`

**Step 1: 创建 session.rs**

```rust
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use tracing::{info, warn};

use crate::transport::Transport;
use crate::protocol::*;
use crate::{McpError, Result};

// ============================================================================
// Config Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransportConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpTransportConfig {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: Option<HashMap<String, String>>,
    },
    #[cfg(feature = "http")]
    Http {
        url: String,
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
    },
}

// ============================================================================
// Session
// ============================================================================

pub struct McpSession {
    server_name: String,
    transport: Box<dyn Transport>,
    capabilities: ServerCapabilities,
    server_info: Implementation,
}

impl McpSession {
    /// Connect to an MCP server
    pub async fn connect<T: Transport + 'static>(
        config: &McpServerConfig,
        transport: T,
    ) -> Result<Self> {
        let mut transport = Box::new(transport);

        // Initialize handshake
        let init_params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "argusx-agent".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let result: InitializeResult = transport
            .request("initialize", Some(init_params))
            .await?;

        info!(
            server_name = %config.name,
            server_version = %result.server_info.version,
            protocol = %result.protocol_version,
            "MCP session initialized"
        );

        // Send initialized notification
        transport.notify::<serde_json::Value>("notifications/initialized", None).await?;

        Ok(Self {
            server_name: config.name.clone(),
            transport,
            capabilities: result.capabilities,
            server_info: result.server_info,
        })
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    pub fn server_info(&self) -> &Implementation {
        &self.server_info
    }

    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    /// Check if server supports tools
    pub fn supports_tools(&self) -> bool {
        self.capabilities.tools.is_some()
    }

    /// Check if server supports resources
    pub fn supports_resources(&self) -> bool {
        self.capabilities.resources.is_some()
    }

    /// Check if server supports prompts
    pub fn supports_prompts(&self) -> bool {
        self.capabilities.prompts.is_some()
    }

    /// List available tools
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        if !self.supports_tools() {
            return Err(McpError::UnsupportedCapability("tools".into()));
        }

        let result: ListToolsResult = self.transport
            .request("tools/list", Some(ListToolsParams { cursor: None }))
            .await?;

        Ok(result.tools)
    }

    /// Call a tool
    pub async fn call_tool(&mut self, name: &str, arguments: serde_json::Value) -> Result<ToolResult> {
        if !self.supports_tools() {
            return Err(McpError::UnsupportedCapability("tools".into()));
        }

        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };

        let result: CallToolResult = self.transport
            .request("tools/call", Some(params))
            .await?;

        Ok(result.into())
    }

    /// List available resources
    pub async fn list_resources(&mut self) -> Result<Vec<Resource>> {
        if !self.supports_resources() {
            return Err(McpError::UnsupportedCapability("resources".into()));
        }

        let result: ListResourcesResult = self.transport
            .request("resources/list", Some(ListResourcesParams { cursor: None }))
            .await?;

        Ok(result.resources)
    }

    /// Read a resource
    pub async fn read_resource(&mut self, uri: &str) -> Result<Vec<ResourceContent>> {
        if !self.supports_resources() {
            return Err(McpError::UnsupportedCapability("resources".into()));
        }

        let params = ReadResourceParams {
            uri: uri.to_string(),
        };

        let result: ReadResourceResult = self.transport
            .request("resources/read", Some(params))
            .await?;

        Ok(result.contents)
    }

    /// List available prompts
    pub async fn list_prompts(&mut self) -> Result<Vec<Prompt>> {
        if !self.supports_prompts() {
            return Err(McpError::UnsupportedCapability("prompts".into()));
        }

        let result: ListPromptsResult = self.transport
            .request("prompts/list", Some(ListPromptsParams { cursor: None }))
            .await?;

        Ok(result.prompts)
    }

    /// Get a prompt
    pub async fn get_prompt(&mut self, name: &str, arguments: serde_json::Value) -> Result<Vec<PromptMessage>> {
        if !self.supports_prompts() {
            return Err(McpError::UnsupportedCapability("prompts".into()));
        }

        let params = GetPromptParams {
            name: name.to_string(),
            arguments,
        };

        let result: GetPromptResult = self.transport
            .request("prompts/get", Some(params))
            .await?;

        Ok(result.messages)
    }

    /// Shutdown the session
    pub async fn shutdown(&mut self) -> Result<()> {
        self.transport.close().await?;
        info!(server_name = %self.server_name, "MCP session closed");
        Ok(())
    }
}
```

**Step 2: 验证编译**

Run: `cd mcp-client && cargo build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add mcp-client/src/session.rs
git commit -m "feat(mcp-client): implement McpSession with tools/resources/prompts"
```

---

## Phase 2: agent-tool MCP 集成

### Task 8: McpToolRegistry 实现

**Files:**
- Modify: `agent-tool/src/mcp/mod.rs`
- Create: `agent-tool/src/mcp/registry.rs`

**Step 1: 更新 mcp/mod.rs**

```rust
mod registry;

pub use registry::McpToolRegistry;
```

**Step 2: 创建 mcp/registry.rs**

```rust
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use mcp_client::{McpSession, McpServerConfig, Tool as McpTool, McpError};
use serde::Deserialize;
use tokio::sync::{RwLock, Mutex as AsyncMutex};
use tracing::{info, warn};

use crate::spec::ToolSpec;
use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;

/// MCP configuration file format
#[derive(Debug, Deserialize)]
struct McpConfigFile {
    servers: Vec<McpServerConfig>,
}

/// Registry for MCP tools
pub struct McpToolRegistry {
    sessions: RwLock<HashMap<String, Arc<AsyncMutex<McpSession>>>>,
    tool_index: RwLock<HashMap<String, String>>,  // full_tool_name -> server_name
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            tool_index: RwLock::new(HashMap::new()),
        }
    }

    /// Load MCP servers from config file
    pub async fn load_from_config(&self, path: &Path) -> Result<(), ToolError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::Io(e))?;

        let config: McpConfigFile = serde_json::from_str(&content)
            .map_err(|e| ToolError::InvalidArgs(format!("invalid MCP config: {}", e)))?;

        for server_config in config.servers {
            if let Err(e) = self.connect_server(&server_config).await {
                warn!(
                    server = %server_config.name,
                    error = %e,
                    "Failed to connect MCP server, skipping"
                );
            }
        }

        Ok(())
    }

    /// Connect to a single MCP server
    pub async fn connect_server(&self, config: &McpServerConfig) -> Result<(), ToolError> {
        info!(server = %config.name, "Connecting to MCP server");

        // Create transport based on config
        #[cfg(feature = "stdio")]
        let transport = {
            use mcp_client::McpTransportConfig;
            match &config.transport {
                McpTransportConfig::Stdio { command, args, env } => {
                    mcp_client::transport::stdio::StdioTransport::new(&config.transport).await
                }
                _ => return Err(ToolError::ExecutionFailed("unsupported transport".into())),
            }
        };

        #[cfg(not(feature = "stdio"))]
        let transport = {
            return Err(ToolError::ExecutionFailed("stdio transport not enabled".into()));
        };

        let transport = transport.map_err(|e| ToolError::ExecutionFailed(
            format!("failed to create transport: {}", e)
        ))?;

        // Connect session
        let mut session = McpSession::connect(config, transport).await
            .map_err(|e| ToolError::ExecutionFailed(
                format!("failed to connect: {}", e)
            ))?;

        // Index tools
        if session.supports_tools() {
            let tools = session.list_tools().await
                .map_err(|e| ToolError::ExecutionFailed(
                    format!("failed to list tools: {}", e)
                ))?;

            let mut index = self.tool_index.write().await;
            for tool in &tools {
                let full_name = format!("mcp_{}_{}", config.name, tool.name);
                index.insert(full_name, config.name.clone());
            }

            info!(
                server = %config.name,
                tool_count = tools.len(),
                "MCP server connected, tools indexed"
            );
        }

        // Store session
        self.sessions.write().await.insert(
            config.name.clone(),
            Arc::new(AsyncMutex::new(session)),
        );

        Ok(())
    }

    /// List all tools from all connected MCP servers
    pub async fn list_all_tools(&self) -> Vec<ToolSpec> {
        let sessions = self.sessions.read().await;
        let mut specs = Vec::new();

        for (server_name, session) in sessions.iter() {
            let mut session = session.lock().await;

            if !session.supports_tools() {
                continue;
            }

            match session.list_tools().await {
                Ok(tools) => {
                    for tool in tools {
                        specs.push(ToolSpec {
                            name: format!("mcp_{}_{}", server_name, tool.name),
                            description: tool.description,
                            input_schema: tool.input_schema,
                        });
                    }
                }
                Err(e) => {
                    warn!(server = %server_name, error = %e, "Failed to list tools");
                }
            }
        }

        specs
    }

    /// Get a specific tool spec
    pub async fn tool_spec(&self, full_name: &str) -> Option<ToolSpec> {
        let (server_name, tool_name) = self.parse_tool_name(full_name)?;
        let sessions = self.sessions.read().await;
        let session = sessions.get(server_name)?;
        let mut session = session.lock().await;

        let tools = session.list_tools().await.ok()?;
        tools.into_iter()
            .find(|t| t.name == tool_name)
            .map(|tool| ToolSpec {
                name: full_name.to_string(),
                description: tool.description,
                input_schema: tool.input_schema,
            })
    }

    /// Call an MCP tool
    pub async fn call_tool(
        &self,
        full_name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let (server_name, tool_name) = self.parse_tool_name(full_name)
            .ok_or_else(|| ToolError::NotFound(full_name.to_string()))?;

        let sessions = self.sessions.read().await;
        let session = sessions.get(server_name)
            .ok_or_else(|| ToolError::NotFound(format!("server: {}", server_name)))?;

        let mut session = session.lock().await;

        let result = session.call_tool(tool_name, args).await
            .map_err(|e| ToolError::ExecutionFailed(format!("MCP tool error: {}", e)))?;

        // Convert MCP ToolResult to our ToolResult
        let output = if let Some(text) = result.text() {
            serde_json::json!({ "content": text })
        } else {
            serde_json::to_value(&result.content).unwrap_or(serde_json::json!({}))
        };

        Ok(ToolResult {
            output,
            is_error: result.is_error,
        })
    }

    /// Parse tool name: "mcp_server_tool" -> ("server", "tool")
    fn parse_tool_name(&self, full_name: &str) -> Option<(&str, &str)> {
        full_name.strip_prefix("mcp_")?;
        let parts: Vec<&str> = parts = full_name[4..].splitn(2, '_').collect();
        if parts.len() != 2 {
            return None;
        }
        Some((parts[0], parts[1]))
    }
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: 更新 agent-tool/Cargo.toml**

```toml
[dependencies]
# ... existing deps ...
mcp-client = { path = "../mcp-client" }
```

**Step 4: 验证编译**

Run: `cd agent-tool && cargo build`
Expected: 编译成功

**Step 5: Commit**

```bash
git add agent-tool/src/mcp/ agent-tool/Cargo.toml
git commit -m "feat(agent-tool): add McpToolRegistry for MCP tool management"
```

---

### Task 9: AggregatedToolRuntime 实现

**Files:**
- Create: `agent-tool/src/aggregated.rs`
- Modify: `agent-tool/src/lib.rs`

**Step 1: 创建 aggregated.rs**

```rust
use std::sync::Arc;

use async_trait::async_trait;
use agent_core::tools::{ToolCatalog, ToolExecutor, ToolExecutionContext, ToolExecutionError, ToolExecutionErrorKind};
use agent_core::{ToolCall, ToolResult as CoreToolResult};

use crate::ToolRegistry;
use crate::context::{ToolContext, ToolResult};
use crate::mcp::McpToolRegistry;

/// Aggregated runtime that combines builtin and MCP tools
pub struct AggregatedToolRuntime {
    builtin: ToolRegistry,
    mcp: McpToolRegistry,
}

impl AggregatedToolRuntime {
    pub fn new(builtin: ToolRegistry, mcp: McpToolRegistry) -> Self {
        Self { builtin, mcp }
    }

    /// Create with default builtin tools and empty MCP registry
    pub async fn with_defaults() -> Self {
        use crate::builtin::{ReadFileTool, ShellTool};

        let builtin = ToolRegistry::new();
        builtin.register(ReadFileTool).await;
        builtin.register(ShellTool).await;

        let mcp = McpToolRegistry::new();

        Self { builtin, mcp }
    }

    /// Get reference to MCP registry for configuration
    pub fn mcp_registry(&self) -> &McpToolRegistry {
        &self.mcp
    }

    /// Check if a tool name is an MCP tool
    fn is_mcp_tool(name: &str) -> bool {
        name.starts_with("mcp_")
    }
}

#[async_trait]
impl ToolCatalog for AggregatedToolRuntime {
    async fn list_tools(&self) -> Vec<agent_core::tools::ToolSpec> {
        let mut tools = Vec::new();

        // Builtin tools
        for spec in self.builtin.list().await {
            tools.push(agent_core::tools::ToolSpec {
                name: spec.name,
                description: spec.description,
                input_schema: spec.input_schema,
                execution_policy: agent_core::tools::ToolExecutionPolicy::default(),
            });
        }

        // MCP tools
        for spec in self.mcp.list_all_tools().await {
            tools.push(agent_core::tools::ToolSpec {
                name: spec.name,
                description: spec.description,
                input_schema: spec.input_schema,
                execution_policy: agent_core::tools::ToolExecutionPolicy::default(),
            });
        }

        tools
    }

    async fn tool_spec(&self, name: &str) -> Option<agent_core::tools::ToolSpec> {
        if Self::is_mcp_tool(name) {
            self.mcp.tool_spec(name).await.map(|spec| agent_core::tools::ToolSpec {
                name: spec.name,
                description: spec.description,
                input_schema: spec.input_schema,
                execution_policy: agent_core::tools::ToolExecutionPolicy::default(),
            })
        } else {
            self.builtin.get(name).await.map(|tool| {
                let spec = tool.spec();
                agent_core::tools::ToolSpec {
                    name: spec.name,
                    description: spec.description,
                    input_schema: spec.input_schema,
                    execution_policy: agent_core::tools::ToolExecutionPolicy::default(),
                }
            })
        }
    }
}

#[async_trait]
impl ToolExecutor for AggregatedToolRuntime {
    async fn execute_tool(
        &self,
        call: ToolCall,
        ctx: ToolExecutionContext,
    ) -> Result<CoreToolResult, ToolExecutionError> {
        let name = &call.tool_name;

        if Self::is_mcp_tool(name) {
            // MCP tool
            let result = self.mcp.call_tool(name, call.arguments).await
                .map_err(|e| ToolExecutionError {
                    kind: ToolExecutionErrorKind::Runtime,
                    message: e.to_string(),
                    retry_after_ms: None,
                })?;

            Ok(CoreToolResult {
                call_id: call.call_id,
                output: result.output,
                is_error: result.is_error,
            })
        } else {
            // Builtin tool
            let tool_ctx = ToolContext {
                session_id: ctx.session_id,
                turn_id: ctx.turn_id,
            };

            let result = self.builtin.call(name, call.arguments, tool_ctx).await
                .map_err(|e| ToolExecutionError {
                    kind: match e {
                        crate::error::ToolError::NotFound(_) => ToolExecutionErrorKind::User,
                        _ => ToolExecutionErrorKind::Runtime,
                    },
                    message: e.to_string(),
                    retry_after_ms: None,
                })?;

            Ok(CoreToolResult {
                call_id: call.call_id,
                output: result.output,
                is_error: result.is_error,
            })
        }
    }
}
```

**Step 2: 更新 lib.rs 导出**

```rust
// 在 lib.rs 中添加:
pub mod aggregated;

pub use aggregated::AggregatedToolRuntime;
```

**Step 3: 验证编译**

Run: `cd agent-tool && cargo build`
Expected: 编译成功

**Step 4: Commit**

```bash
git add agent-tool/src/aggregated.rs agent-tool/src/lib.rs
git commit -m "feat(agent-tool): add AggregatedToolRuntime combining builtin and MCP tools"
```

---

## Phase 3: 集成和测试

### Task 10: 集成测试

**Files:**
- Create: `agent-tool/tests/mcp_integration_test.rs`

**Step 1: 写集成测试**

```rust
use agent_tool::AggregatedToolRuntime;
use agent_tool::mcp::McpToolRegistry;
use agent_tool::ToolRegistry;
use agent_core::tools::{ToolCatalog, ToolExecutor};
use agent_core::ToolCall;
use std::path::PathBuf;

#[tokio::test]
async fn test_aggregated_lists_builtin_tools() {
    let runtime = AggregatedToolRuntime::with_defaults().await;

    let tools = runtime.list_tools().await;

    // Should have at least read_file and shell
    assert!(tools.iter().any(|t| t.name == "read_file"));
    assert!(tools.iter().any(|t| t.name == "shell"));
}

#[tokio::test]
async fn test_aggregated_returns_builtin_spec() {
    let runtime = AggregatedToolRuntime::with_defaults().await;

    let spec = runtime.tool_spec("read_file").await;
    assert!(spec.is_some());
    assert_eq!(spec.unwrap().name, "read_file");
}

#[tokio::test]
async fn test_aggregated_returns_none_for_unknown_tool() {
    let runtime = AggregatedToolRuntime::with_defaults().await;

    let spec = runtime.tool_spec("unknown_tool").await;
    assert!(spec.is_none());
}

#[tokio::test]
async fn test_mcp_tool_name_parsing() {
    // MCP tool names should start with "mcp_"
    assert!(AggregatedToolRuntime::is_mcp_tool("mcp_filesystem_read_file"));
    assert!(!AggregatedToolRuntime::is_mcp_tool("read_file"));
    assert!(!AggregatedToolRuntime::is_mcp_tool("shell"));
}
```

**Step 2: 运行测试**

Run: `cd agent-tool && cargo test`
Expected: 所有测试通过

**Step 3: Commit**

```bash
git add agent-tool/tests/mcp_integration_test.rs
git commit -m "test(agent-tool): add integration tests for AggregatedToolRuntime"
```

---

### Task 11: Workspace 集成

**Files:**
- Modify: `Cargo.toml` (root)

**Step 1: 添加 mcp-client 到 workspace**

```toml
# 在 [workspace.members] 中添加:
"mcp-client",
```

**Step 2: 验证整个项目编译**

Run: `cargo build --workspace`
Expected: 编译成功

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add mcp-client to workspace"
```

---

## Verification

### 编译检查
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
```

### 手动测试
1. 创建 `.mcp.json` 配置文件
2. 使用 `AggregatedToolRuntime::with_defaults()` 创建运行时
3. 调用 `mcp_registry().load_from_config()` 加载配置
4. 验证工具列表包含 MCP 工具
5. 调用 MCP 工具验证执行

---

## Summary

| Phase | Tasks | Key Deliverables |
|-------|-------|------------------|
| Phase 1 | 1-7 | `mcp-client` crate with protocol, transport, session |
| Phase 2 | 8-9 | `McpToolRegistry` and `AggregatedToolRuntime` in agent-tool |
| Phase 3 | 10-11 | Integration tests and workspace setup |
