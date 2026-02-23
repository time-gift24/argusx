# Agent Tool Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create an `agent-tool` crate that provides Tool abstraction for the agent system, supporting built-in tools (file, shell) and MCP integration.

**Architecture:**
- Independent crate `agent-tool` under workspace
- Define `Tool` trait similar to Codex's `ToolHandler` pattern
- Support built-in tools (file operations, shell execution)
- MCP client adapter for external tool integration

**Tech Stack:**
- Rust
- async-trait for async trait support
- serde_json for JSON handling
- tokio for async runtime

---

## Task 1: Create agent-tool crate scaffold

**Files:**
- Create: `agent-tool/Cargo.toml`
- Create: `agent-tool/src/lib.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "agent-tool"
version = "0.1.0"
edition = "2021"

[dependencies]
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tokio = { version = "1.0", features = ["sync"] }
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1.0", features = ["test-util", "rt-multi-thread"] }
```

**Step 2: Create lib.rs with module declarations**

```rust
pub mod error;
pub mod spec;
pub mod context;
pub mod trait_def;
pub mod registry;
pub mod builtin;
```

**Step 3: Commit**

```bash
mkdir -p agent-tool/src
git add agent-tool/Cargo.toml agent-tool/src/lib.rs
git commit -m "feat(agent-tool): scaffold agent-tool crate"
```

---

## Task 2: Define error types

**Files:**
- Create: `agent-tool/src/error.rs`

**Step 1: Write the failing test**

```rust
// In agent-tool/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

**Step 2: Run test to verify it compiles**

Run: `cargo check -p agent-tool`
Expected: PASS (no test yet, just checking compilation)

**Step 3: Commit**

```bash
git add agent-tool/src/error.rs
git commit -m "feat(agent-tool): add error types"
```

---

## Task 3: Define ToolContext and ToolResult

**Files:**
- Create: `agent-tool/src/context.rs`

**Step 1: Write the context module**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub session_id: String,
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: serde_json::Value,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(output: serde_json::Value) -> Self {
        Self {
            output,
            is_error: false,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            output: serde_json::json!({ "error": message.into() }),
            is_error: true,
        }
    }
}
```

**Step 2: Commit**

```bash
git add agent-tool/src/context.rs
git commit -m "feat(agent-tool): add ToolContext and ToolResult"
```

---

## Task 4: Define ToolSpec (LLM-facing)

**Files:**
- Create: `agent-tool/src/spec.rs`

**Step 1: Write the spec module**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}
```

**Step 2: Commit**

```bash
git add agent-tool/src/spec.rs
git commit -m "feat(agent-tool): add ToolSpec for LLM"
```

---

## Task 5: Define Tool trait

**Files:**
- Create: `agent-tool/src/trait_def.rs`

**Step 1: Write the failing test**

```rust
// In agent-tool/src/trait_def.rs
use async_trait::async_trait;
use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn spec(&self) -> ToolSpec;

    async fn execute(
        &self,
        ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError>;
}
```

**Step 2: Run test to verify it compiles**

Run: `cargo check -p agent-tool`
Expected: PASS (trait definition)

**Step 3: Commit**

```bash
git add agent-tool/src/trait_def.rs
git commit -m "feat(agent-tool): add Tool trait"
```

---

## Task 6: Implement ToolRegistry

**Files:**
- Create: `agent-tool/src/registry.rs`

**Step 1: Write the registry**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::ToolContext;
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, tool: impl Tool + 'static) {
        let tool = Arc::new(tool);
        self.tools.write().await.insert(tool.name().to_string(), tool);
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<ToolSpec> {
        let tools = self.tools.read().await;
        tools.values().map(|t| t.spec()).collect()
    }

    pub async fn call(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let tool = self.get(name).await.ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(ctx, args).await
    }
}
```

**Step 2: Commit**

```bash
git add agent-tool/src/registry.rs
git commit -m "feat(agent-tool): add ToolRegistry"
```

---

## Task 7: Create builtin tools module

**Files:**
- Create: `agent-tool/src/builtin/mod.rs`
- Create: `agent-tool/src/builtin/file.rs`

**Step 1: Write builtin file tool**

```rust
use async_trait::async_trait;
use serde_json::json;

use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read contents of a file"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path is required".to_string()))?;

        let content = tokio::fs::read_to_string(path).await?;
        Ok(ToolResult::ok(json!({ "content": content })))
    }
}
```

**Step 2: Commit**

```bash
git add agent-tool/src/builtin/mod.rs agent-tool/src/builtin/file.rs
git commit -m "feat(agent-tool): add builtin file tools"
```

---

## Task 8: Add builtin shell tool

**Files:**
- Create: `agent-tool/src/builtin/shell.rs`

**Step 1: Write builtin shell tool**

```rust
use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;

use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command is required".to_string()))?;

        let output = tokio::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .await?;

        let result = json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code(),
        });

        Ok(ToolResult::ok(result))
    }
}
```

**Step 2: Commit**

```bash
git add agent-tool/src/builtin/shell.rs
git commit -m "feat(agent-tool): add builtin shell tool"
```

---

## Task 9: Update lib.rs exports

**Files:**
- Modify: `agent-tool/src/lib.rs`

**Step 1: Update lib.rs**

```rust
pub mod error;
pub mod spec;
pub mod context;
pub mod trait_def;
pub mod registry;
pub mod builtin;

pub use error::ToolError;
pub use spec::ToolSpec;
pub use context::{ToolContext, ToolResult};
pub use trait_def::Tool;
pub use registry::ToolRegistry;
```

**Step 2: Run test**

Run: `cargo check -p agent-tool`
Expected: PASS

**Step 3: Commit**

```bash
git add agent-tool/src/lib.rs
git commit -m "feat(agent-tool): export public API"
```

---

## Task 10: Add MCP module placeholder

**Files:**
- Create: `agent-tool/src/mcp/mod.rs`

**Step 1: Write MCP module**

```rust
// MCP client integration - placeholder for future implementation
// Will integrate with @modelcontextprotocol/sdk equivalent in Rust

pub mod client {
    // Placeholder for MCP client implementation
}
```

**Step 2: Commit**

```bash
git add agent-tool/src/mcp/mod.rs
git commit -m "feat(agent-tool): add MCP module placeholder"
```

---

## Task 11: Add workspace dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add to workspace members**

```toml
[workspace]
members = [
    # ... existing members
    "agent-tool",
]
```

**Step 2: Run test**

Run: `cargo check -p agent-tool`
Expected: PASS

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add agent-tool to workspace"
```

---

## Task 12: Integration test

**Files:**
- Create: `agent-tool/tests/integration_test.rs`

**Step 1: Write integration test**

```rust
use agent_tool::{ToolRegistry, ReadFileTool, ShellTool, ToolContext, ToolSpec};

#[tokio::test]
async fn test_registry_register_and_list() {
    let registry = ToolRegistry::new();
    registry.register(ReadFileTool).await;
    registry.register(ShellTool).await;

    let tools = registry.list().await;
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().any(|t| t.name == "read_file"));
    assert!(tools.iter().any(|t| t.name == "shell"));
}

#[tokio::test]
async fn test_registry_get_tool() {
    let registry = ToolRegistry::new();
    registry.register(ReadFileTool).await;

    let tool = registry.get("read_file").await;
    assert!(tool.is_some());
}
```

**Step 2: Run test**

Run: `cargo test -p agent-tool`
Expected: PASS

**Step 3: Commit**

```bash
git add agent-tool/tests/integration_test.rs
git commit -m "test(agent-tool): add integration tests"
```

---

**Plan complete and saved to `docs/plans/2026-02-22-agent-tool-plan.md`**

Two execution options:

1. **Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

2. **Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?
