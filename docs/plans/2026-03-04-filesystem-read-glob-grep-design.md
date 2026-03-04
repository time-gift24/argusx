# Internal Filesystem (Read/Glob/Grep) Design

**Date:** 2026-03-04  
**Status:** Approved  
**Scope:** `agent-tool` only (do not modify `agent-core` protocol)

## 1. Background and Goal

Current `agent-tool` builtins are:
- `read_file` (direct path read, no guard)
- `shell` (arbitrary command execution)

Target state:
- Expose only three read-only tools: `read`, `glob`, `grep`
- Remove/disable `shell` and all write capabilities
- Keep `agent-core` and runtime event protocol unchanged
- Build an internal filesystem capability layer with strict path sandboxing

## 2. Research Summary

### 2.1 Upstream Node `@modelcontextprotocol/server-filesystem` (v0.6.3)

Observed in `src/filesystem`:
- Tool surface includes read/write/edit/search/list/move
- Security model uses allowed directories + path normalization + `realpath` checks
- Includes extensive path/symlink/race-condition tests
- `search_files` supports glob; no first-class content grep tool

### 2.2 `rust-mcp-filesystem` rewrite (v0.4.0)

Observed in `.vendor/rust-mcp-filesystem`:
- Richer tool surface, including:
  - `search_files_content` (content grep/regex)
  - `read_file_lines`, `head_file`, `tail_file`
  - extra read analytics tools
- Uses `grep` crate ecosystem for content search
- Uses `walkdir` + glob matching + allowed roots checks

### 2.3 Delta Relevant to This Design

Compared to Node upstream:
- Rust rewrite adds content grep and more granular read modes
- Both implementations center on allowed-root sandboxing
- Node tests capture important symlink/race threat models we should keep

## 3. Requirement Freeze

Confirmed with stakeholder:
1. Tool names: only `read`, `glob`, `grep` (no compatibility aliases)
2. Disable `shell`
3. Disable all write-class operations
4. Include enhanced read capabilities in v1:
   - pagination, head/tail, batch read, metadata/list mode
   - glob filters and limits
   - grep regex/options/context
5. Grep backend: internal Rust implementation using ripgrep crate ecosystem; no external `rg` dependency in v1

## 4. Architecture

## 4.1 Layering

`Tool.execute(args)`  
-> `FsGuard.resolve_and_authorize(...)`  
-> `FsEngine.operation(...)`  
-> `ToolResult { output, is_error }`

## 4.2 New Internal Modules (`agent-tool`)

Planned module tree:

```text
agent-tool/src/builtin/
  mod.rs
  read.rs
  glob.rs
  grep.rs
  fs/
    mod.rs
    guard.rs
    engine.rs
    types.rs
    error.rs
```

Responsibilities:
- `guard.rs`: path normalization, root boundary, symlink-safe resolution, deny traversal
- `engine.rs`: shared filesystem operations for all three tools
- `types.rs`: request/response payload structs
- `error.rs`: filesystem-domain errors and conversion helpers
- `read.rs/glob.rs/grep.rs`: tool façade (schema + call into engine)

## 4.3 Runtime Registration Changes

`AgentToolRuntime::default_with_builtins()` will register:
- `read`
- `glob`
- `grep`

Will not register:
- `shell`
- `read_file`

`agent-core` interfaces remain unchanged.

## 5. Tool Contracts

## 5.1 `read`

Single tool with mode-based behavior:
- `mode: "text" | "lines" | "head" | "tail" | "stat" | "list" | "batch"`

Core fields:
- `path` (for all modes except batch may allow `paths`)
- `with_line_numbers` (text/lines/head/tail)
- `offset` + `limit` (lines)
- `count` (head/tail)
- `paths` (batch)
- `sort_by` (`name|size|mtime`) for list

## 5.2 `glob`

Fields:
- `path`
- `pattern`
- `exclude_patterns`
- `max_depth`
- `max_results`
- `min_bytes`, `max_bytes`

Output:
- Matched entries (path + basic type/size)
- Meta info with truncation/scanned counts

## 5.3 `grep`

Fields:
- `path` (file or directory root)
- `pattern` (target file glob)
- `query`
- `is_regex`
- `case_sensitive`
- `whole_word`
- `before_context`, `after_context`
- `exclude_patterns`
- `max_results`

Output per match:
- file path
- line number
- column/start position
- snippet and optional context lines

## 6. Security Model

All IO must go through `FsGuard`.

## 6.1 Path Controls

- Expand `~`
- Convert relative path to absolute path using configured roots
- Normalize path (remove `.` and traversal effects)
- Enforce membership in `allowed_roots`
- For existing paths: `realpath` and re-check root membership
- For non-existing target: resolve/check real parent directory

## 6.2 Symlink Policy

- Allow symlink only when resolved target remains inside allowed roots
- Deny symlink escape with explicit access-denied error

## 6.3 Prefix Attack Prevention

Use boundary-safe path checks (avoid `/allowed` matching `/allowed_evil`).

## 6.4 Read-Only Enforcement

- No write operations exposed in tool registry
- No shell command execution path exposed
- Filesystem module should avoid write APIs entirely in v1

## 7. Grep Backend Decision

v1 backend: internal Rust implementation via ripgrep ecosystem crates (e.g., `grep`, `grep-regex`, `grep-searcher`, `ignore`, `globset`).

Reason:
- No external binary dependency
- Better cross-platform packaging consistency
- Stable performance and mature API

Cross-platform support status:
- Linux: mature
- macOS: mature
- Windows: mature with known caveats handled in tests (CRLF, path separators, symlink privileges, encoding edge cases)

## 8. Error Model

Unified tool error codes:
- `E_FS_INVALID_ARGS`
- `E_FS_NOT_FOUND`
- `E_FS_ACCESS_DENIED`
- `E_FS_UNSUPPORTED`
- `E_FS_LIMIT_EXCEEDED`
- `E_FS_IO`

Runtime mapping:
- user-caused input/security errors -> `ToolExecutionErrorKind::User`
- transient backend issues (if any) -> `Transient`
- other IO/runtime failures -> `Runtime`

## 9. Limits and Performance

Default guardrails:
- max bytes per read
- max matches/results per glob/grep
- bounded recursion depth
- bounded context lines

Meta fields in outputs:
- `truncated`
- `scanned_files`
- `matched_files`
- `elapsed_ms`

## 10. Test Strategy and Acceptance

## 10.1 Unit Tests

- `FsGuard`:
  - root allow/deny
  - traversal deny
  - symlink in-root/out-root
  - non-existent path parent validation
- `read` modes:
  - text, lines, head, tail, stat, list, batch
- `glob`:
  - include/exclude, depth, limits, byte filters
- `grep`:
  - literal vs regex, case sensitivity, whole word, context, truncation

## 10.2 Integration Tests

- Runtime exposes exactly `read/glob/grep`
- `shell` not found
- representative success/failure flows through runtime adapter

## 10.3 Security Regression Tests

Carry over threat cases inspired by upstream tests:
- symlink escape denial
- similar-prefix path denial
- post-validation path mutation safety checks (read-path race boundaries)

## 10.4 Acceptance Gate

Ship-ready only when:
1. `cargo test -p agent-tool` passes
2. No write or shell tool is registered in default runtime
3. Three tools satisfy schema and behavior contracts above

## 11. Non-Goals (v1)

- No compatibility aliases for old tool names
- No write operations
- No external `rg` binary invocation fallback

## 12. Rollout Notes

1. Keep changes isolated to `agent-tool`
2. Land tests first, then implementation
3. Enable by default once contract tests pass
