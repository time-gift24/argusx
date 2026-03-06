# Tool Runtime Redesign

Date: 2026-03-06
Status: Approved
Scope: Architecture and contract design only

## 1. Goal

Replace the stale `agent-tool` crate with a new `tool` runtime aligned with the current `core::ResponseEvent` and `core::ToolCall` contract.

The redesign must:

- rename `agent-tool` to `tool`
- extend `core::ToolCall` to model builtin tools explicitly
- keep MCP as a first-class tool target with a concrete runtime implementation
- support per-agent `.toml` configuration for enabled builtin tools
- support per-tool and per-MCP-server concurrency limits and timeout policy overrides

## 2. Context

Current state in the workspace:

- [`core`](/Users/wanyaozhong/Projects/argusx/core/src/lib.rs) already treats tool calls as first-class response events via `ResponseEvent::ToolDone(ToolCall)`.
- `core::ToolCall` currently models only `FunctionCall` and `Mcp`.
- [`agent-tool`](/Users/wanyaozhong/Projects/argusx/agent-tool/src/lib.rs) still uses an older runtime contract and still references the removed `agent-core` API shape.
- [`agent-tool/src/mcp/mod.rs`](/Users/wanyaozhong/Projects/argusx/agent-tool/src/mcp/mod.rs) is still a placeholder.

This redesign is therefore a contract and runtime consolidation effort, not just a crate rename.

## 3. Approved Decisions

1. `core::ToolCall` will be expanded together with the `tool` runtime redesign.
2. Builtin tools are modeled as a concrete enum, not broad capability buckets.
3. Each agent has a `.toml` file that selects builtin tools and overrides runtime policy.
4. Agent config is override-based, not full tool redefinition.
5. MCP and builtin tools share one outer scheduler, but keep separate concrete executors underneath.
6. MCP is implemented concretely now, with `stdio` transport in v1.

## 4. Approach Options

### Option 1: Strongly Typed Unified Contract

Extend `core::ToolCall`, make `tool` the unified runtime entry point, and keep builtin/MCP execution behind separate executors.

Pros:

- response events carry the real tool type directly
- easy to validate and test across crates
- configuration and concurrency rules are explicit

Cons:

- requires coordinated edits across `core`, `provider`, and `tool`

### Option 2: String-Driven Runtime Mapping

Keep the contract more generic and classify builtin tools mostly at runtime based on names and config.

Pros:

- flexible

Cons:

- weaker typing
- more runtime-only failures
- does not match the approved decision to extend `core::ToolCall` clearly

### Option 3: Late Resolution in Tool Runtime

Leave provider output mostly as `FunctionCall` and let the runtime reinterpret selected names as builtin or MCP later.

Pros:

- smaller provider changes

Cons:

- event semantics stay ambiguous
- loses the benefit of explicit tool typing in `ResponseEvent`

### Selected Approach

Option 1 is approved.

## 5. Target Architecture

### 5.1 Crate Boundaries

- `core`: stable cross-crate tool call facts
- `tool`: runtime catalog, config, scheduling, builtin execution, MCP execution
- `provider`: classify streamed tool calls into builtin, MCP, or generic function calls as early as possible

### 5.2 Runtime Shape

`tool` becomes a unified runtime facade with:

- `config` module
- `catalog` module
- `scheduler` module
- `builtin` module
- `mcp` module
- `error` module

The outer runtime accepts `core::ToolCall` and dispatches internally:

- `ToolCall::Builtin` -> builtin executor
- `ToolCall::Mcp` -> MCP executor
- `ToolCall::FunctionCall` -> unsupported in v1, reserved for later

### 5.3 Response Contract

`ResponseEvent` stays unchanged at the event name level. Only the `ToolDone(ToolCall)` payload gains more precision through the expanded `ToolCall` variants.

## 6. Core Contract Changes

The target shape is:

```rust
pub enum ToolCall {
    FunctionCall {
        sequence: u32,
        call_id: String,
        name: String,
        arguments_json: String,
    },
    Builtin(BuiltinToolCall),
    Mcp(McpCall),
}

pub struct BuiltinToolCall {
    pub sequence: u32,
    pub call_id: String,
    pub builtin: Builtin,
    pub arguments_json: String,
}

pub enum Builtin {
    Read,
    Glob,
    Grep,
    UpdatePlan,
    Shell,
    DomainCookies,
    Unknown(String),
}
```

Design notes:

- `ToolCall` represents the model-requested tool invocation fact, not runtime policy.
- Concurrency, timeout, and permission rules do not belong in `core::ToolCall`.
- `Builtin::Unknown(String)` exists for forward compatibility, but agent config loading should reject unknown builtin names by default.

## 7. Agent Configuration Model

Each agent owns a `.toml` file that enables a builtin tool set and optionally overrides execution policy.

Example shape:

```toml
[tools]
builtin_tools = ["read", "glob", "grep", "update_plan"]

[tools.defaults]
allow_parallel = true
max_concurrency = 4
timeout_ms = 5000

[tools.builtin.read]
max_concurrency = 16
allowed_roots = ["."]
mode = "text_or_binary"

[tools.builtin.grep]
max_concurrency = 8

[tools.builtin.update_plan]
allow_parallel = false

[mcp.defaults]
allow_parallel = true
max_concurrency = 4
timeout_ms = 10000

[mcp.server.filesystem]
enabled = true
transport = "stdio"
command = "uvx"
args = ["mcp-server-filesystem", "/workspace"]
max_concurrency = 2
```

Rules:

- `tools.builtin_tools` is the enablement whitelist
- builtin override blocks can only change runtime policy and implementation parameters
- builtin names, schema, and enum identity remain code-defined
- `allow_parallel = false` forces effective concurrency to `1`
- effective values merge as `agent override > config defaults > code defaults`
- unknown builtin names, invalid concurrency, and overrides for disabled builtin tools are configuration errors

## 8. Scheduling and Concurrency

### 8.1 Effective Policy

Builtin and MCP execution both use the same outer policy model:

- `allow_parallel`
- `max_concurrency`
- `timeout_ms`

The scheduler computes an effective policy per target:

- per builtin for builtin tools
- per server label for MCP

### 8.2 Runtime Behavior

The scheduler uses bounded permits rather than immediate rejection:

- if a target is serial-only, it gets one permit
- if a target is parallel-capable, it gets `max_concurrency` permits
- callers wait for a permit

This keeps model-emitted bursts safe while still respecting configured limits.

### 8.3 Scope of v1

v1 limits only per-target concurrency. A cross-tool global concurrency ceiling is intentionally out of scope.

## 9. Builtin Tool Model

Builtin tool metadata is code-defined, not config-defined.

Each builtin definition contains:

- canonical name
- description
- input schema
- default execution policy
- concrete executor implementation

The config layer only selects and overrides these definitions for a given agent.

This preserves semantic stability for provider integration and tool catalog generation.

## 10. MCP Runtime Design

### 10.1 v1 Transport

MCP is implemented concretely with `stdio` transport only in v1.

### 10.2 Lifecycle

The MCP runtime manages `server_label -> client/session`.

Connection strategy is lazy:

- first tool discovery or first tool call starts the server process
- sessions are reused per configured server

### 10.3 Exposed Capabilities

The MCP runtime must support:

- `list_tools(server_label)`
- `call_tool(server_label, tool_name, arguments_json)`

Future transports such as SSE or HTTP can be added behind transport-specific modules without changing `core::ToolCall`.

## 11. Provider Integration

Provider should classify tool calls early:

- known builtin canonical names -> `ToolCall::Builtin`
- MCP payloads -> `ToolCall::Mcp`
- everything else -> `ToolCall::FunctionCall`

This keeps streamed response events aligned with the runtime dispatch model and avoids a second ambiguous name-based classification step later.

## 12. Error Model

Three error layers are required:

- `ConfigError`: invalid agent config, unknown builtin name, invalid concurrency values, override of disabled tool
- `DispatchError`: disabled builtin, unknown MCP server, unsupported `FunctionCall`
- `ExecutionError`: builtin failure, MCP startup failure, MCP protocol failure, timeout

Policy:

- configuration errors fail fast during startup or config reload
- execution failures return structured tool errors without crashing the runtime

## 13. Testing Strategy

Required coverage:

- `core`: shape tests for `ToolCall::Builtin`
- `provider`: builtin call name upgrades into `ToolCall::Builtin`
- `tool/config`: `.toml` parsing, merge rules, invalid config failures
- `tool/scheduler`: serial execution and bounded parallel execution behavior
- `tool/mcp`: mock `stdio` server integration for `list_tools` and `call_tool`
- regressions: unknown builtin names are not silently accepted; unknown MCP servers return structured errors

## 14. Non-Goals

This design explicitly excludes:

- global cross-tool concurrency caps
- live hot-reload of agent `.toml`
- MCP transports other than `stdio`
- a concrete executor for generic `FunctionCall`

## 15. Migration Direction

1. Extend `core::ToolCall` and add builtin shape tests.
2. Rename `agent-tool` to `tool` and remove the old `agent-core` coupling.
3. Introduce config loading and effective policy merge logic.
4. Add the unified scheduler with per-target concurrency control.
5. Implement builtin executors behind the new runtime.
6. Implement concrete `stdio` MCP support.
7. Update provider mapping to emit builtin calls explicitly.
8. Add integration tests across `core`, `provider`, and `tool`.

## 16. Approved Scope

This document reflects the reviewed decisions:

- `core::ToolCall` expands together with the redesign
- builtin tools are concrete enum variants
- each agent selects builtin tools and overrides policy through `.toml`
- MCP and builtin share one outer scheduler but use separate executors
- MCP v1 is concrete and `stdio`-only
