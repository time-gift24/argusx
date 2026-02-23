# Agent CLI (ratatui) Single-Session Multi-Turn Design

**Date:** 2026-02-23  
**Status:** Confirmed  
**Owner:** Runtime team

## 1. Goal

Create a new `agent-cli` binary using `ratatui` for chat-first terminal UX.

Required behavior:
- Launch directly into chat mode (no subcommand).
- Default startup creates a new session automatically.
- Passing `--session <session_id>` resumes that existing session.
- Single-session only in v1 (no in-app session switching).
- Multi-turn conversation persists through the same `session_id`.

## 2. Scope

In scope:
- New workspace crate `agent-cli`.
- `ratatui` + `crossterm` interactive UI.
- Streaming assistant text display.
- Reasoning display with fold/unfold control.
- Tool call progress display.
- Session bootstrap policy:
  - no `--session`: create new session;
  - with `--session`: validate and resume.

Out of scope:
- Multi-session switching UI.
- Session CRUD pages in TUI.
- Data migration.
- Replacing existing `agent-turn-cli` / `agent-session-cli` flows.

## 3. Architecture and Boundaries

`agent-cli` only owns terminal interaction and event orchestration. Business execution and persistence remain in existing crates.

Responsibilities split:
- `agent` facade: model runtime orchestration and session persistence.
- `agent-cli`: CLI args, session bootstrap, UI rendering, event loop.

Runtime path:
1. Parse CLI args.
2. Build `Agent` via `AgentBuilder`.
3. Resolve session:
   - `--session` provided -> `get_session` must exist.
   - not provided -> `create_session`.
4. Enter TUI event loop.
5. For each user input, call `chat_stream(session_id, message)`.

This keeps state ownership clear and avoids duplicating transcript logic already handled by `agent-session`.

## 4. Components and Responsibilities

`agent-cli` modules:

1. `cli.rs`
- Define args and defaults.
- Key args: `--api-key`, `--base-url`, `--model`, `--store-dir`, `--session`, `--system-prompt`, `--max-tokens`, `--temperature`, `--top-p`, `--debug-events`.

2. `app.rs`
- Define `AppState`:
  - `session_id`
  - message timeline
  - input buffer
  - reasoning fold state
  - tool progress map
  - active streaming turn state

3. `runtime.rs`
- Wrap `Agent` operations.
- Convert `AgentStreamEvent` into internal `AppEvent`.
- Enforce one active turn at a time in UI layer.

4. `ui.rs`
- Pure rendering (history panel + input panel + status bar).
- Render reasoning folded/unfolded.
- Render tool progress states.

5. `event_loop.rs`
- Central async loop for:
  - keyboard events
  - runtime stream events
  - redraw ticks
- Handles exit and cleanup.

## 5. Data Flow and State Management

Session bootstrap:
- If `--session <id>`:
  - call `get_session(id)`;
  - if missing, print actionable error and exit.
- Else:
  - call `create_session` and store returned id.

Message send flow:
1. User presses Enter.
2. If an active turn exists, reject input with warning message.
3. Append user message to timeline.
4. Start `chat_stream` for same `session_id`.
5. Consume stream events and incrementally update current assistant turn.

Event mapping:
- `UiThreadEvent::MessageDelta` -> append assistant text.
- `UiThreadEvent::ReasoningDelta` -> append reasoning buffer.
- `UiThreadEvent::ToolCallRequested/Progress/Completed` -> update tool progress.
- `UiThreadEvent::Warning/Error` -> add system note.
- `RunStreamEvent::TurnDone/TurnFailed` -> close active turn.

The next turn reuses same `session_id`, so transcript recovery remains delegated to `agent-session`.

## 6. Error Handling and Edge Cases

Startup errors (fatal):
- missing API key;
- invalid/resolution failure of `--session`;
- model initialization failure.

Runtime errors (recoverable):
- `chat_stream` start failure -> render system error, keep app alive.
- stream failure/turn failure -> close turn with failed status; allow next input.
- tool call failure -> mark specific tool failed; do not force crash.

Concurrency guard:
- block Enter while one turn is active.
- if backend still returns busy, surface a user-friendly warning.

Exit behavior:
- `Esc` or `Ctrl+C` gracefully restores terminal state.
- active stream task is cancelled on shutdown.

## 7. UX and Keybindings (v1)

Default layout:
- Top: scrollable chat timeline.
- Bottom: single-line (or wrapped) input box.
- Status bar: model/session and connection state.

Keybindings:
- `Enter`: send message.
- `Tab`: toggle reasoning fold/unfold.
- `PgUp/PgDn` (or `Ctrl+U`/`Ctrl+D` fallback): scroll history.
- `Esc` / `Ctrl+C`: quit.

Display policy:
- Show user and assistant messages.
- Show streaming assistant deltas.
- Show reasoning section (foldable).
- Show tool progress compactly (`planned/running/completed/failed`).

## 8. Testing Strategy

1. Unit tests (`app.rs`)
- input submit transitions;
- stream event application;
- tool progress state transitions;
- turn close/reopen behavior.

2. Component tests (`ui.rs` with `ratatui::backend::TestBackend`)
- layout region allocation;
- reasoning fold visibility;
- tool progress rendering labels.

3. Integration tests (`runtime` + mock model)
- two-turn same-session flow;
- `--session` resume path;
- failed turn followed by successful next turn.

Verification commands:
- `cargo test -p agent-cli`
- `cargo clippy -p agent-cli --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`

## 9. Rollout and Migration

Rollout strategy: additive and non-breaking.

Phase 1:
- Introduce new `agent-cli` crate.
- Keep existing `agent-turn-cli` and `agent-session-cli` unchanged.

Phase 2 (optional, future):
- Mark `agent-cli` as recommended interactive entry.
- Keep low-level CLIs as debugging/automation tools.

No storage migration is required because `agent-cli` reuses current session persistence.

## 10. Acceptance Criteria

- `agent-cli` launches directly into chat UI.
- Default launch creates a new session.
- `--session <id>` resumes existing session and continues multi-turn chat.
- Reasoning can be folded/unfolded.
- Tool call progress is visible during turn execution.
- Single-session constraint is enforced in-app.
- All `agent-cli` tests and checks pass.
