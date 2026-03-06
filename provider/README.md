# provider

Unified streaming provider crate with internal dialects:
- `Dialect::Openai`
- `Dialect::Zai`

Both dialects emit the same external contract: `core::ResponseEvent`.

## Usage

```rust
use provider::{Dialect, Mapper};

let mut mapper = Mapper::new(Dialect::Openai);
let events = mapper.feed(raw_chunk_json)?;
let final_events = mapper.on_done()?;
```

For Z.AI payloads:

```rust
use provider::{Dialect, Mapper};

let mut mapper = Mapper::new(Dialect::Zai);
let events = mapper.feed(raw_chunk_json)?;
let final_events = mapper.on_done()?;
```

## MCP Mapping Contract

Tool calls are classified as MCP when either condition is true:
- `tool_call.type == "mcp"`
- function name starts with `__mcp__`

MCP payload parsing:
- Parses `arguments_json` as Z.AI MCP JSON
- Parse failure is a hard protocol error (`Error::...::Protocol`)

Non-MCP calls are emitted as `ToolCall::FunctionCall`.

## Stream Safety

- Tool calls are emitted in `sequence` order.
- Terminal contract is strict: after terminal event, further `feed/on_done` returns error.
- Duplicate tool sequences from mixed `delta.tool_calls` and `message.tool_calls` are deduplicated.
