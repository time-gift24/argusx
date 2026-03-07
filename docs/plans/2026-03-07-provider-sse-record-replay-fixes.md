# Provider SSE Record/Replay Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Preserve raw SSE frames during live recording and make replay follow the same SSE parsing semantics as live streams.

**Architecture:** Keep `ReplayReader` as the timed raw-frame source, but feed replay frames back through `eventsource_stream` so comment frames and non-`data:` fields are handled the same way as live traffic. Extend the vendored SSE event type with raw frame text so live recording writes the original frame instead of reconstructing it from `message.data`.

**Tech Stack:** Rust 2024, Tokio, reqwest, async-stream, vendored `eventsource_stream`, provider integration tests.

---

### Task 1: Add regression tests for raw recording and replay semantics

**Files:**
- Modify: `.worktrees/provider-sse-record-replay/provider/tests/provider_record_replay_roundtrip_test.rs`
- Modify: `.worktrees/provider-sse-record-replay/provider/tests/provider_replay_mode_test.rs`

**Step 1: Write the failing tests**

- Add a live-recording test that sends SSE frames containing `id:` / `event:` fields and asserts the saved `.sse` file preserves those raw lines.
- Add a replay-mode test that starts from a raw `.sse` file containing a comment-only heartbeat frame and asserts replay still emits `Created` and `Done` instead of failing on the heartbeat.

**Step 2: Run tests to verify they fail**

Run: `cargo test -p provider live_recording_preserves_raw_sse_fields replay_mode_ignores_comment_only_frames`

Expected: the raw-recording assertion fails because the saved file only contains reconstructed `data:` frames, and the replay heartbeat test fails with a terminal parse error.

### Task 2: Preserve raw frames in live recording and align replay parsing

**Files:**
- Modify: `.worktrees/provider-sse-record-replay/vendor/eventsource_stream/src/event.rs`
- Modify: `.worktrees/provider-sse-record-replay/vendor/eventsource_stream/src/event_stream.rs`
- Modify: `.worktrees/provider-sse-record-replay/provider/src/client.rs`

**Step 1: Implement the minimal fix**

- Extend the vendored SSE `Event` with raw frame text while keeping its equality behavior stable for existing tests.
- Populate that raw frame text when an event is dispatched from the parser.
- In provider live mode, record `message.raw` instead of reformatting `message.data`.
- In replay mode, convert `ReplayReader` frames back into an SSE byte stream and run them through `eventsource_stream` before mapping to provider payloads.

**Step 2: Run focused tests**

Run: `cargo test -p provider live_recording_preserves_raw_sse_fields replay_mode_ignores_comment_only_frames`

Expected: both tests pass.

### Task 3: Verify regressions and commit

**Files:**
- Modify: `.worktrees/provider-sse-record-replay/provider/tests/provider_stream_failure_test.rs` only if expectations need updating

**Step 1: Run the relevant suites**

Run: `cargo test -p provider`

Run: `cargo test -p turn tracing_turn_test`

**Step 2: Commit**

```bash
git -C /Users/wanyaozhong/projects/argusx/.worktrees/provider-sse-record-replay add \
  docs/plans/2026-03-07-provider-sse-record-replay-fixes.md \
  provider/src/client.rs \
  provider/tests/provider_record_replay_roundtrip_test.rs \
  provider/tests/provider_replay_mode_test.rs \
  vendor/eventsource_stream/src/event.rs \
  vendor/eventsource_stream/src/event_stream.rs
git -C /Users/wanyaozhong/projects/argusx/.worktrees/provider-sse-record-replay commit -m "fix(provider): preserve raw sse frames in record and replay"
```
