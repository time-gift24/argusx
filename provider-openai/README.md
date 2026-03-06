# provider-openai

OpenAI-compatible provider adapter for Argus streaming responses.

## Usage

```rust
use provider_openai::Mapper;

let mut mapper = Mapper::new("openai".into());

// Feed SSE chunks
for chunk in sse_chunks {
    let events = mapper.feed(chunk).unwrap();
    for event in events {
        // Process ResponseEvent::Created, ContentDelta, ReasoningDelta, ToolDone, etc.
    }
}

// Signal completion
let final_events = mapper.on_done().unwrap();
```

## Event Contract

- `Created` - emitted exactly once on first chunk
- `ContentDelta` - incremental content updates
- `ReasoningDelta` - incremental reasoning/thinking updates
- `ToolDelta` - incremental tool call argument updates
- `ToolDone` - emitted when finish_reason is "tool_calls"
- `Done` - emitted on [DONE] signal, may contain Usage

## Fixture Replay

Run fixture tests:

```bash
cargo test -p provider-openai --test fixture_replay_test
```

## Known Out-of-Scope

- HTTP client / network transport
- Authentication / API key management
- Request building (prompt → OpenAI format)
- Integration with existing llm-client/llm-provider path
