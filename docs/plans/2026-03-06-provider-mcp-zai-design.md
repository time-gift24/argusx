# Provider Unified Dialects + Z.AI MCP-First Design

Date: 2026-03-06
Status: Approved and implemented on branch `core-provider-openai-streaming`
Scope: Architecture and contract design only (no implementation in this document)

## 1. Goal

Unify provider parsing into a single `provider` crate with two internal dialects (`openai`, `zai`) and one external event contract (`core::ResponseEvent`).

MCP is a first-class tool call. OpenAI compatibility is retained as baseline.

## 2. Locked Decisions

1. Crate strategy
- One crate: `provider`
- Internal dialect modules: `dialect::openai` and `dialect::zai`

2. MCP routing and naming
- Internal MCP marker prefix: `__mcp__{name}`
- MCP classification rule:
- `tool_call.type == "mcp"` OR function name has prefix `__mcp__`

3. MCP payload source of truth
- Use Z.AI MCP JSON shape as internal MCP data model
- For prefixed function calls in OpenAI dialect, parse `arguments_json` as Z.AI MCP JSON

4. Error policy
- If MCP JSON parsing fails: return `Protocol` error immediately

5. Event contract target
- Keep external stream output as `core::ResponseEvent`
- Break old `core::ToolCall::Mcp` shape and replace with Z.AI-aligned definition

## 3. Core Contract Changes

`core::ResponseEvent` shape remains unchanged.

`core::ToolCall` changes:

```rust
pub enum ToolCall {
    FunctionCall {
        sequence: u32,
        call_id: String,
        name: String,
        arguments_json: String,
    },
    Mcp(ZaiMcpCall),
}

pub struct ZaiMcpCall {
    pub sequence: u32,
    pub id: String,
    pub mcp_type: ZaiMcpType,     // mcp_list_tools | mcp_call
    pub server_label: Option<String>,
    pub name: Option<String>,
    pub arguments_json: Option<String>,
    pub output_json: Option<String>,
    pub tools_json: Option<String>,
    pub error: Option<String>,
}

pub enum ZaiMcpType {
    McpListTools,
    McpCall,
    Unknown(String),
}
```

Notes:
- `sequence` remains required to preserve deterministic tool emission order.
- `id` is the tool-call ID, mapped from provider payload (`call_id` source).
- Optional fields remain optional because stream chunks are often partial.

## 4. Provider Architecture

## 4.1 Module Layout

- `provider/src/lib.rs`
- `provider/src/error.rs`
- `provider/src/mapper.rs` (unified state machine + contract)
- `provider/src/normalize/tool_calls.rs` (shared tool normalization)
- `provider/src/dialect/openai/{schema.rs,parser.rs,mapper.rs}`
- `provider/src/dialect/zai/{schema.rs,parser.rs,mapper.rs}`

## 4.2 Unified External API

- `Mapper::new(dialect: Dialect)`
- `feed(raw: &str) -> Result<Vec<ResponseEvent>, Error>`
- `on_done() -> Result<Vec<ResponseEvent>, Error>`

## 4.3 Unified Stream State Machine

Shared state in one place:
- created/meta emission guard
- content/reasoning buffers
- pending tool-call buffer (ordered by `sequence`)
- usage cache
- terminal state guard

Terminal invariants:
- exactly one terminal event (`Done(_)` or `Error(_)`)
- no event allowed after terminal

## 5. Tool Normalization Rules

All dialect-specific chunks are normalized into one internal pending structure:

```rust
struct PendingToolCallNormalized {
    sequence: u32,
    call_id: String,
    call_type: Option<String>,
    name: Option<String>,
    arguments_json: String,
}
```

Routing:
1. `call_type == "mcp"` => MCP path
2. Else if `name` starts with `__mcp__` => MCP path
3. Else => `FunctionCall` path

MCP path:
- Parse `arguments_json` as Z.AI MCP JSON
- On parse failure => `Protocol` error
- Build `ToolCall::Mcp(ZaiMcpCall)`

Function path:
- Keep existing behavior and ordering checks

## 6. Dialect-Specific Behavior

OpenAI dialect:
- Continue supporting standard Chat Completions stream chunks
- Add MCP upgrade path for `__mcp__{name}` function calls

Z.AI dialect:
- Support stream chunks (`chat.completion.chunk`)
- Support terminal full completion payloads (`chat.completion`) that carry `message.tool_calls`
- Merge both paths into same normalized tool assembly state

## 7. Compatibility and Migration

1. New `provider` crate becomes canonical implementation.
2. Existing `provider-openai` becomes temporary compatibility wrapper or is removed after migration.
3. Call sites consuming `core::ToolCall::Mcp { server, method, payload_json }` must migrate to `Mcp(ZaiMcpCall)`.

## 8. Testing Strategy

Core tests:
- `ToolCall::Mcp(ZaiMcpCall)` shape
- contract terminal invariants unchanged

Provider tests:
- openai fixture replay unchanged behavior for non-MCP calls
- openai `__mcp__` prefixed function call -> `ToolCall::Mcp`
- zai fixture replay with `delta.tool_calls` and `message.tool_calls`
- ordered tool emission by sequence
- invalid MCP JSON -> `Protocol` error
- done usage finalization remains correct

## 9. Out of Scope

- MCP client runtime implementation
- message multimodal normalization beyond current chat completion requirements
- changing existing business semantics outside tool-call mapping and provider crate unification

## 10. Implementation Status

- Implemented: unified `provider` crate with `openai` and `zai` dialects.
- Implemented: MCP-first mapping using `__mcp__` prefix and `type == "mcp"` classifier.
- Implemented: strict MCP JSON parse failure -> protocol error.
- Implemented: fixture replay tests for both openai and zai dialects.
- Note: no external workspace call sites depended on `provider-openai`; migration step was a no-op in this branch.
