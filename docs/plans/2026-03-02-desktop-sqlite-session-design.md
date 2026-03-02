# Desktop SQLite Session Design

## 1. Background

Current desktop chat state is primarily persisted on the frontend (`zustand persist`), while backend Tauri session commands are still partial/mock for list/read paths. This causes state split and weak startup restore semantics.

## 2. Goals

1. Desktop uses a single SQLite database as the only source of truth for session/turn/transcript data.
2. Replace previous desktop session persistence logic end-to-end (no frontend full-history persistence).
3. On startup, load sessions ordered by:
   - primary: latest completed LLM turn `ended_at` descending
   - fallback: `session.updated_at` descending when `ended_at` is absent
4. Auto-select the first session on startup.
5. Default load window is only the most recent 24 hours of data.
6. Full history is user-triggered via a top-left "Full Info" entry point.
7. Frontend cache is memory-only with a hard global budget of 64MB.

## 3. Non-Goals

1. No migration of non-desktop entry points (`agent`, `agent-cli`) in this phase.
2. No changes to model/provider runtime config feature scope.
3. No broad cross-product storage unification beyond desktop path.

## 4. Architecture

### 4.1 Runtime Layer

Use `SessionRuntime` as the business orchestration entry. Keep behavior (create/list/restore/run turn) but swap desktop storage backend to SQLite implementation.

### 4.2 Storage Layer

Refactor `agent-session` storage behind existing store trait boundaries, then add `SqliteSessionStore` for desktop wiring.

Desktop Tauri app initializes runtime with SQLite-backed store only.

### 4.3 Frontend Layer

`chat-store` becomes memory-only runtime state:

1. Session index (lightweight metadata)
2. Active session recent data (24h window)
3. UI-only transient state (selection, draft, dialog state, cursors)

No full history persistence in browser storage.

## 5. SQLite Schema

### 5.1 `sessions`

- `session_id TEXT PRIMARY KEY`
- `title TEXT NOT NULL`
- `status TEXT NOT NULL` (`active|idle|archived`)
- `created_at_ms INTEGER NOT NULL`
- `updated_at_ms INTEGER NOT NULL`
- `archived_at_ms INTEGER NULL`

### 5.2 `turns`

- `turn_id TEXT PRIMARY KEY`
- `session_id TEXT NOT NULL`
- `epoch INTEGER NOT NULL`
- `started_at_ms INTEGER NOT NULL`
- `ended_at_ms INTEGER NULL`
- `status TEXT NOT NULL` (`running|done|failed|cancelled`)
- `final_message TEXT NULL`
- `tool_calls_count INTEGER NOT NULL DEFAULT 0`
- `input_tokens INTEGER NOT NULL DEFAULT 0`
- `output_tokens INTEGER NOT NULL DEFAULT 0`

### 5.3 `transcript_items`

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `session_id TEXT NOT NULL`
- `turn_id TEXT NOT NULL`
- `seq INTEGER NOT NULL`
- `item_type TEXT NOT NULL`
- `payload_json TEXT NOT NULL`
- `UNIQUE(turn_id, seq)`

### 5.4 Indexes

- `idx_turns_session_ended(session_id, ended_at_ms DESC)`
- `idx_turns_session_started(session_id, started_at_ms DESC)`
- `idx_transcript_turn_seq(turn_id, seq)`
- `idx_transcript_session_turn(session_id, turn_id)`

## 6. Query Semantics

### 6.1 Session List Ordering

For each session compute:

- `last_ended_at = MAX(turns.ended_at_ms)`

Sort:

1. `last_ended_at DESC` (nulls last)
2. `sessions.updated_at_ms DESC`

### 6.2 Default Data Window

When opening session by default:

- load only data in `[now - 24h, now]`
- older data excluded from initial memory footprint

### 6.3 Full Info Loading

"Full Info" entry loads older/full data only on user action, paginated by cursor.

## 7. Desktop API Contract Changes

1. `list_chat_sessions` returns real SQLite-backed list including `last_ended_at`.
2. `get_chat_messages(session_id, range, cursor)` supports:
   - `range = "last_24h"` (default)
   - `range = "all"` with pagination
3. `get_session_overview(session_id)` provides total stats/time span for full-info modal.
4. create/delete/update session commands become SQLite writes.

## 8. Frontend Cache and Memory Policy

### 8.1 Hard Limit

- Global chat cache hard limit: `64MB`.

### 8.2 Cached Objects

1. Session lightweight metadata
2. Active session recent messages/turn view-models
3. Minimal preview state for non-active sessions

### 8.3 Eviction Policy

On overflow, evict by LRU:

1. Non-active session detailed messages/turns first
2. Then old active-session details (keep summary/preview)

Also cap per-turn large fields (reasoning/tool/terminal text) with truncation.

## 9. Startup and UX Behavior

1. Desktop startup:
   - query sorted session list
   - auto-select first session
   - load only 24h window for selected session
2. Chat top-left includes "Full Info" trigger:
   - opens modal
   - user explicitly requests full history/pages
   - modal-scoped data can be released on close

## 10. Error Handling

1. SQLite init/migration failure is surfaced explicitly; no silent fallback to frontend full persistence.
2. Read failures are localized (retryable UI) and do not poison in-memory cache.
3. Turn completion write path (`turn summary + session updated_at`) is transactional.
4. Transcript persistence is idempotent by `(turn_id, seq)` uniqueness.

## 11. Migration and Compatibility

1. Desktop stops using previous `chat-storage` as authoritative history state.
2. On first rollout, desktop can clear legacy chat persistence key to avoid stale hydration.
3. Schema migration managed via `PRAGMA user_version`.

## 12. Test Plan

### 12.1 `agent-session` SQLite Store Tests

1. create/get/update/delete/list lifecycle
2. list ordering: `last_ended_at` + `updated_at` fallback
3. transcript append/load ordering and pagination

### 12.2 Desktop Tauri Integration Tests

1. startup auto-selects sorted first session
2. default 24h-only load
3. full-info modal triggers explicit full-range paginated fetch

### 12.3 Frontend Store Tests

1. no persistent full-history hydration
2. 64MB limit enforcement and LRU eviction behavior
3. session switch reload/eviction correctness

## 13. Rollout Notes

1. Implement behind desktop-scoped storage wiring first.
2. Validate with realistic long-history data to verify memory cap behavior.
3. Keep non-desktop entry points unchanged in this phase.
