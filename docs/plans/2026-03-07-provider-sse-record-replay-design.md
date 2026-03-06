# Provider SSE Record/Replay Design

Date: 2026-03-07
Status: Approved
Scope: Provider-side raw SSE recording/replay plus tracing design only

## 1. Goal

Add a dev-oriented recording and replay capability for provider streams while also improving diagnostics with `tracing`.

This design must:

- keep [`ProviderClient::stream()`](/Users/wanyaozhong/projects/argusx/provider/src/client.rs) as the single external stream entry point
- preserve raw SSE text as the primary recording artifact
- support dev-only replay from file through the same provider contract
- support both fast replay and recorded-timing replay
- add tracing at both `provider` and `turn` runtime boundaries
- keep recorder failures from breaking the main streaming path

This phase is architecture and contract design only. It does not implement production policy, UI changes, or prompt/tool redaction.

## 2. Context

Current workspace context:

- [`provider/src/client.rs`](/Users/wanyaozhong/projects/argusx/provider/src/client.rs) owns the live HTTP + SSE pull path and maps incoming messages into `ResponseEvent`.
- [`core/src/lib.rs`](/Users/wanyaozhong/projects/argusx/core/src/lib.rs) already defines the stable provider-to-runtime contract via `ResponseStream` and `ResponseEvent`.
- [`turn/src/driver.rs`](/Users/wanyaozhong/projects/argusx/turn/src/driver.rs) consumes `ResponseEvent` and should remain agnostic to whether events came from live HTTP or replayed files.
- [`provider/tests/fixtures`](/Users/wanyaozhong/projects/argusx/provider/tests/fixtures) already contains raw SSE-like fixture text used to validate mapper behavior.

Implication:

- the correct abstraction boundary is inside `provider`, not `turn`
- replay should feed the same mapper path as live SSE instead of inventing a dev-only event bypass
- tracing and recording must be designed as separate observability mechanisms with different responsibilities

## 3. Approved Inputs From Discussion

1. The saved artifact is raw SSE text, not only normalized `ResponseEvent`.
2. Replay is selected at the `provider` layer while callers continue to use the same `stream()` API.
3. Local dev recordings are the default working asset; valuable captures can later be promoted into committed fixtures.
4. Replay must support both fast and recorded-timing modes.
5. Live recording is explicit opt-in in dev mode, not automatic for every request.
6. Tracing should cover the full chain conceptually, with `provider` as the required implementation anchor and `turn` as the follow-on runtime boundary.

## 4. Approaches Considered

### Option 1: Keep Everything in `provider/src/client.rs`

Extend the existing client file to handle live SSE, replay, recording, timing simulation, and tracing directly.

Pros:

- smallest immediate file diff
- preserves current call sites

Cons:

- mixes transport, recording, replay, and diagnostics responsibilities
- raises maintenance cost quickly
- weakens unit-test boundaries

### Option 2: Add Internal Provider Source/Recorder/Replay Roles

Keep `ProviderClient::stream()` stable while introducing internal abstractions for source selection, recording, and replay.

Pros:

- clean separation of live transport vs dev replay
- recorder stays optional and side-effect-only
- easiest path to test and evolve

Cons:

- introduces a few extra types and files

### Option 3: Build a Separate Dev Replay Runtime Outside `provider`

Move replay into a distinct crate or external dev-only adapter.

Pros:

- strongest isolation from production HTTP code

Cons:

- over-engineered for the current workspace size
- leaks live/replay concerns upward
- duplicates provider contract handling

### Selected Approach

Option 2 is approved.

## 5. Target Architecture

The target provider architecture is:

- `ProviderClient` remains the only public orchestration entry point
- a source layer produces canonical raw SSE frames
- a recorder layer optionally mirrors live frames to disk
- a replay layer reads stored frames and emits them as if they were live
- the existing mapper still converts raw frame payloads into `ResponseEvent`
- `turn` remains unaware of live vs replay mode

Recommended internal roles:

- `LiveSseSource`
- `ReplayFileSource`
- `SseRecorder`
- `ReplayReader`

Recommended file split:

- [`provider/src/client.rs`](/Users/wanyaozhong/projects/argusx/provider/src/client.rs): orchestration only
- `provider/src/source.rs`: live/replay source definitions
- `provider/src/record.rs`: raw SSE recorder
- `provider/src/replay.rs`: replay reader and timing logic

The exact file names may change slightly during implementation, but the responsibilities must remain separated.

## 6. Provider Configuration Model

The external provider API should remain structurally simple while allowing dev behavior injection.

Recommended concepts:

```rust
enum ProviderStreamMode {
    Live,
    Replay {
        file_path: PathBuf,
        timing: ReplayTiming,
    },
}

enum ReplayTiming {
    Fast,
    Recorded,
}

struct ProviderDevOptions {
    stream_mode: ProviderStreamMode,
    record_live_sse: Option<RecordTarget>,
}
```

Design rules:

- default behavior is `Live`
- replay is selected explicitly by configuration, not heuristics
- live recording is disabled by default and enabled explicitly
- dev configuration is consumed inside `provider`; `turn` does not branch on it

This design intentionally avoids creating a second public stream API for replay.

## 7. Recording File Format

The recording format is a two-part model:

### 7.1 Primary Artifact

The primary artifact is a raw SSE text file using the existing fixture-friendly shape:

```text
data: {...}

data: {...}

data: [DONE]
```

Notes:

- the saved representation is canonical SSE frame text, not arbitrary raw TCP bytes
- this keeps recordings portable and aligned with existing fixture expectations
- the canonical text file is the asset that may later be promoted into [`provider/tests/fixtures`](/Users/wanyaozhong/projects/argusx/provider/tests/fixtures)

### 7.2 Optional Sidecar

Recorded-timing replay uses an optional sidecar file such as `capture.sse.meta.json`.

The sidecar may store:

- frame index
- relative timestamp offset
- dialect
- recording start timestamp
- source URL or host

Rules:

- replay without sidecar still works in `Fast` mode
- `Recorded` mode requires valid timing metadata
- timing metadata does not pollute the canonical SSE text file

## 8. Data Flow

### 8.1 Live Flow

1. `ProviderClient::stream()` creates the provider root span and resolves `ProviderStreamMode`.
2. `LiveSseSource` performs the HTTP request and yields canonical SSE frames.
3. If live recording is enabled, `SseRecorder` writes the frame text and optional timing metadata as a side effect.
4. The frame payload continues into the existing dialect mapper.
5. The mapper emits `ResponseEvent`.
6. [`turn/src/driver.rs`](/Users/wanyaozhong/projects/argusx/turn/src/driver.rs) consumes those events unchanged.

### 8.2 Replay Flow

1. `ProviderClient::stream()` resolves `Replay { file_path, timing }`.
2. `ReplayReader` loads the `.sse` file and optional sidecar metadata.
3. It emits canonical SSE frames into the same mapper path used by live mode.
4. If timing is `Recorded`, replay waits between frames according to the sidecar offsets.
5. If timing is `Fast`, frames are emitted without delay.

Key rule:

- replay does not bypass the mapper and does not synthesize `ResponseEvent` directly

## 9. Tracing Design

Tracing and recording are complementary but separate:

- recording preserves replayable protocol content
- tracing preserves diagnostics, lifecycle, and correlation

### 9.1 Provider Tracing

One provider stream span per request is recommended, conceptually named `provider.stream`.

Suggested fields:

- `dialect`
- `mode`
- `model`
- `base_url` or host
- `replay_file` when applicable
- `timing` when applicable
- `record_enabled`

Suggested events:

- request started
- HTTP response accepted
- SSE frame received
- mapper emitted event count
- recorder wrote frame
- replay frame emitted
- `[DONE]` received
- transport, parse, protocol, or replay error
- stream completed

Rules:

- log frame index, byte size, and event counts by default
- do not log entire SSE payloads by default
- on errors, include only bounded payload summaries when needed

### 9.2 Turn Tracing

One turn span per runtime invocation is recommended, conceptually named `turn.run`.

Suggested fields:

- `turn_id`
- `session_id`
- `provider_dialect`
- `stream_mode`

Suggested events:

- turn started
- step started and finished
- tool batch prepared
- permission requested and resolved
- tool completed
- turn finished, cancelled, or failed

Rules:

- provider tracing explains stream transport and mapping
- turn tracing explains runtime interpretation and orchestration
- replay debugging should first identify the failure in tracing, then reopen the saved SSE capture

## 10. Error Handling and Degradation

The design distinguishes four failure classes:

### 10.1 Configuration Errors

Examples:

- missing replay file path
- non-existent replay file
- invalid or unreadable sidecar metadata for `Recorded` mode

Handling:

- fail before building the stream
- return `Err(...)` from `ProviderClient::stream()`

### 10.2 Source Runtime Errors

Examples:

- HTTP status failure
- invalid content-type
- transport and SSE parser failures
- replay file read failure after stream start

Handling:

- after the stream is established, terminate through the existing `ResponseEvent::Error` contract

### 10.3 Mapper Errors

Handling:

- identical behavior for live and replay
- stop the stream and emit `ResponseEvent::Error`
- include frame index and bounded diagnostic context in tracing

### 10.4 Recorder Errors

Handling:

- recorder failure is non-fatal by default
- the live stream continues
- tracing records a `warn`

Rationale:

- recording is a dev support feature and must not destabilize the primary path

## 11. Testing Strategy

The implementation should cover five groups of tests:

1. Source tests
   - keep existing wiremock validation for live SSE
   - add replay-source tests for `.sse` input
   - cover both `Fast` and `Recorded`
2. Recorder tests
   - verify canonical SSE file output
   - verify optional sidecar contents and monotonic offsets
3. Provider integration tests
   - `Replay` mode should produce the same mapper-level `ResponseEvent` sequence shape as fixture-driven expectations
   - cover at least OpenAI and Zai dialects
4. Degradation tests
   - recorder write failure does not break stream completion
   - `Recorded` mode without valid timing metadata fails explicitly
5. Tracing smoke tests
   - verify key spans/events exist
   - avoid brittle full-string log snapshot assertions

## 12. Acceptance Criteria

The design is considered successfully implemented when:

- `ProviderClient::stream()` remains the single public stream API
- dev can explicitly enable live SSE recording
- recordings default to a local path and can later be promoted into committed fixtures
- replay accepts recorded `.sse` files directly
- replay supports both `Fast` and `Recorded`
- live and replay both pass through the same mapper path
- `provider` and `turn` both expose useful tracing boundaries
- recorder failures degrade safely without breaking the main flow
- at least one end-to-end capture-to-fixture replay workflow is proven by tests
