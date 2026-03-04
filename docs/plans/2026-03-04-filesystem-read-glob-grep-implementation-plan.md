# Internal Read/Glob/Grep Filesystem Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace current `agent-tool` builtins with strict read-only `read`, `glob`, `grep` tools backed by internal filesystem guards and ripgrep crates, while keeping `agent-core` unchanged.

**Architecture:** Implement a shared `fs` layer (`FsGuard` + `FsEngine`) inside `agent-tool`, then expose three tool façades (`read`, `glob`, `grep`) that all route through the guard before any I/O. Remove default registrations for `shell` and legacy `read_file`.

**Tech Stack:** Rust (`tokio` async I/O), `grep` crate ecosystem (`grep`, `grep-regex`, `grep-searcher`, `ignore`, `globset`), `walkdir`, `serde_json`, existing `agent-core` tool interfaces.

---

### Task 1: Lock Runtime Tool Surface Contract

**Files:**
- Modify: `agent-tool/tests/runtime_adapter_test.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn default_builtins_expose_only_read_glob_grep() {
    use agent_core::tools::ToolCatalog;
    let rt = agent_tool::AgentToolRuntime::default_with_builtins().await;
    let mut names = rt
        .list_tools()
        .await
        .into_iter()
        .map(|t| t.name)
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, vec!["glob", "grep", "read"]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool default_builtins_expose_only_read_glob_grep -- --nocapture`  
Expected: FAIL because current runtime still exposes `read_file` and `shell`.

**Step 3: Write minimal implementation**

No implementation yet; keep failure as red baseline.

**Step 4: Run test to verify it still fails for the right reason**

Run: `cargo test -p agent-tool default_builtins_expose_only_read_glob_grep -- --nocapture`  
Expected: FAIL with mismatch in tool names.

**Step 5: Commit**

```bash
git add agent-tool/tests/runtime_adapter_test.rs
git commit -m "test(agent-tool): add builtin surface contract for read/glob/grep"
```

### Task 2: Add Filesystem Core Skeleton

**Files:**
- Create: `agent-tool/src/builtin/fs/mod.rs`
- Create: `agent-tool/src/builtin/fs/error.rs`
- Create: `agent-tool/src/builtin/fs/types.rs`
- Create: `agent-tool/src/builtin/fs/guard.rs`
- Create: `agent-tool/src/builtin/fs/engine.rs`
- Modify: `agent-tool/src/builtin/mod.rs`

**Step 1: Write the failing compile usage test**

```rust
// in a new test module
use agent_tool::builtin::fs::guard::FsGuard;
#[test]
fn fs_guard_type_is_exposed() {
    let _ = std::any::type_name::<FsGuard>();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool fs_guard_type_is_exposed -- --nocapture`  
Expected: FAIL due to missing module/types.

**Step 3: Write minimal implementation**

```rust
// fs/mod.rs
pub mod engine;
pub mod error;
pub mod guard;
pub mod types;
```

```rust
// fs/guard.rs
pub struct FsGuard;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-tool fs_guard_type_is_exposed -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-tool/src/builtin/fs agent-tool/src/builtin/mod.rs agent-tool/tests
git commit -m "feat(agent-tool): scaffold internal fs core modules"
```

### Task 3: Implement and Verify `FsGuard` Path Security

**Files:**
- Create: `agent-tool/tests/fs_guard_test.rs`
- Modify: `agent-tool/src/builtin/fs/guard.rs`
- Modify: `agent-tool/src/builtin/fs/error.rs`

**Step 1: Write failing security tests**

```rust
#[tokio::test]
async fn guard_denies_path_outside_allowed_roots() { /* ... */ }

#[tokio::test]
async fn guard_denies_dotdot_traversal_escape() { /* ... */ }

#[tokio::test]
async fn guard_denies_symlink_escape_target() { /* ... */ }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-tool fs_guard_test -- --nocapture`  
Expected: FAIL due to unimplemented guard behavior.

**Step 3: Write minimal implementation**

```rust
pub struct FsGuard { /* allowed_roots */ }
impl FsGuard {
    pub fn new(allowed_roots: Vec<std::path::PathBuf>) -> Self { /* ... */ }
    pub async fn authorize_existing(&self, path: &str) -> Result<std::path::PathBuf, FsError> { /* normalize + realpath + root check */ }
    pub async fn authorize_maybe_new(&self, path: &str) -> Result<std::path::PathBuf, FsError> { /* parent realpath check */ }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool fs_guard_test -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-tool/tests/fs_guard_test.rs agent-tool/src/builtin/fs/guard.rs agent-tool/src/builtin/fs/error.rs
git commit -m "feat(agent-tool): add guarded path authorization for allowed roots"
```

### Task 4: Add `read` Tool Contract Tests

**Files:**
- Create: `agent-tool/tests/read_tool_test.rs`
- Create: `agent-tool/src/builtin/read.rs`
- Modify: `agent-tool/src/builtin/mod.rs`

**Step 1: Write failing tests for key read modes**

```rust
#[tokio::test]
async fn read_text_mode_returns_full_content() { /* ... */ }

#[tokio::test]
async fn read_head_tail_and_lines_modes_work() { /* ... */ }

#[tokio::test]
async fn read_stat_and_list_modes_work() { /* ... */ }

#[tokio::test]
async fn read_batch_mode_returns_per_path_results() { /* ... */ }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool read_tool_test -- --nocapture`  
Expected: FAIL due to missing `read` tool.

**Step 3: Write minimal implementation**

```rust
pub struct ReadTool;
#[async_trait::async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str { "read" }
    fn description(&self) -> &str { "Read-only filesystem operations (text, lines, head/tail, stat, list, batch)." }
    fn spec(&self) -> ToolSpec { /* mode-based input schema */ }
    async fn execute(&self, ctx: ToolContext, args: serde_json::Value) -> Result<ToolResult, ToolError> { /* route by mode */ }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool read_tool_test -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-tool/tests/read_tool_test.rs agent-tool/src/builtin/read.rs agent-tool/src/builtin/mod.rs
git commit -m "feat(agent-tool): add read tool with multi-mode read-only operations"
```

### Task 5: Add `glob` Tool (Pattern + Filters + Limits)

**Files:**
- Create: `agent-tool/tests/glob_tool_test.rs`
- Create: `agent-tool/src/builtin/glob.rs`
- Modify: `agent-tool/src/builtin/fs/engine.rs`
- Modify: `agent-tool/src/builtin/mod.rs`

**Step 1: Write failing glob tests**

```rust
#[tokio::test]
async fn glob_matches_pattern_and_excludes_paths() { /* ... */ }

#[tokio::test]
async fn glob_honors_max_depth_and_max_results() { /* ... */ }

#[tokio::test]
async fn glob_honors_min_max_bytes_filters() { /* ... */ }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool glob_tool_test -- --nocapture`  
Expected: FAIL due to missing `glob` tool.

**Step 3: Write minimal implementation**

```rust
pub struct GlobTool;
impl Tool for GlobTool { /* name=glob */ }
// engine: walk files safely + apply globset/include-exclude + limits
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool glob_tool_test -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-tool/tests/glob_tool_test.rs agent-tool/src/builtin/glob.rs agent-tool/src/builtin/fs/engine.rs agent-tool/src/builtin/mod.rs
git commit -m "feat(agent-tool): add glob tool with depth/size/result limits"
```

### Task 6: Add `grep` Tool via ripgrep Crates

**Files:**
- Modify: `agent-tool/Cargo.toml`
- Create: `agent-tool/tests/grep_tool_test.rs`
- Create: `agent-tool/src/builtin/grep.rs`
- Modify: `agent-tool/src/builtin/fs/engine.rs`
- Modify: `agent-tool/src/builtin/fs/types.rs`
- Modify: `agent-tool/src/builtin/mod.rs`

**Step 1: Write failing grep tests**

```rust
#[tokio::test]
async fn grep_literal_and_regex_both_work() { /* ... */ }

#[tokio::test]
async fn grep_supports_case_whole_word_and_context() { /* ... */ }

#[tokio::test]
async fn grep_honors_max_results_and_sets_truncated_meta() { /* ... */ }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool grep_tool_test -- --nocapture`  
Expected: FAIL because grep backend/tool is missing.

**Step 3: Write minimal implementation**

```rust
// Cargo deps: grep, grep-regex, grep-searcher, ignore, globset
pub struct GrepTool;
impl Tool for GrepTool { /* name=grep */ }
// engine: file selection + searcher sink + structured match output
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool grep_tool_test -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-tool/Cargo.toml agent-tool/tests/grep_tool_test.rs agent-tool/src/builtin/grep.rs agent-tool/src/builtin/fs/engine.rs agent-tool/src/builtin/fs/types.rs agent-tool/src/builtin/mod.rs
git commit -m "feat(agent-tool): add grep tool using ripgrep crate ecosystem"
```

### Task 7: Remove Legacy `shell` and `read_file` from Default Runtime

**Files:**
- Modify: `agent-tool/src/runtime.rs`
- Modify: `agent-tool/src/builtin/mod.rs`
- Modify: `agent-tool/tests/runtime_adapter_test.rs`
- Modify: `agent-tool/tests/integration_test.rs`

**Step 1: Write failing negative tests**

```rust
#[tokio::test]
async fn default_runtime_rejects_shell_tool() { /* execute_tool("shell") expects user error */ }

#[tokio::test]
async fn default_runtime_rejects_read_file_tool() { /* execute_tool("read_file") expects user error */ }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool default_runtime_rejects_shell_tool default_runtime_rejects_read_file_tool -- --nocapture`  
Expected: FAIL while legacy tools still exist in default registration.

**Step 3: Write minimal implementation**

```rust
pub async fn default_with_builtins() -> Self {
    let registry = ToolRegistry::new();
    registry.register(ReadTool).await;
    registry.register(GlobTool).await;
    registry.register(GrepTool).await;
    Self { registry }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool runtime_adapter -- --nocapture`  
Expected: PASS on runtime contract tests.

**Step 5: Commit**

```bash
git add agent-tool/src/runtime.rs agent-tool/src/builtin/mod.rs agent-tool/tests/runtime_adapter_test.rs agent-tool/tests/integration_test.rs
git commit -m "refactor(agent-tool): switch default builtins to read/glob/grep only"
```

### Task 8: Unify Error Mapping and Output Shape

**Files:**
- Modify: `agent-tool/src/error.rs`
- Modify: `agent-tool/src/runtime.rs`
- Modify: `agent-tool/src/builtin/fs/error.rs`
- Modify: `agent-tool/tests/*_tool_test.rs`

**Step 1: Write failing mapping tests**

```rust
#[tokio::test]
async fn access_denied_maps_to_user_error_kind() { /* ... */ }

#[tokio::test]
async fn io_failure_maps_to_runtime_error_kind() { /* ... */ }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool access_denied_maps_to_user_error_kind io_failure_maps_to_runtime_error_kind -- --nocapture`  
Expected: FAIL on current mapping.

**Step 3: Write minimal implementation**

```rust
// add explicit fs error variants and mapping table into ToolExecutionErrorKind
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool -- --nocapture`  
Expected: PASS for mapping tests and prior suites.

**Step 5: Commit**

```bash
git add agent-tool/src/error.rs agent-tool/src/runtime.rs agent-tool/src/builtin/fs/error.rs agent-tool/tests
git commit -m "feat(agent-tool): normalize filesystem error codes and runtime mapping"
```

### Task 9: Final Verification + Docs Update

**Files:**
- Modify: `agent-tool/README.md` (or nearest docs file if exists)
- Modify: `docs/plans/2026-03-04-filesystem-read-glob-grep-design.md` (status/checklist section)

**Step 1: Add failing doc assertion checklist item**

```markdown
- [ ] default runtime only exposes read/glob/grep
```

**Step 2: Run full verification**

Run:
- `cargo test -p agent-tool`
- `cargo test -p agent-core`

Expected: all PASS.

**Step 3: Update docs to final state**

```markdown
- [x] read/glob/grep only
- [x] shell removed from default runtime
- [x] write operations unavailable
```

**Step 4: Re-run verification**

Run:
- `cargo test -p agent-tool`
- `cargo test -p agent-core`

Expected: all PASS.

**Step 5: Commit**

```bash
git add agent-tool/README.md docs/plans/2026-03-04-filesystem-read-glob-grep-design.md
git commit -m "docs(agent-tool): document read/glob/grep-only filesystem runtime"
```

## Implementation Notes

- Keep each task DRY and YAGNI: no write-path abstractions, no compatibility aliases.
- Apply TDD strictly per task.
- Use frequent small commits; do not batch multiple tasks into one commit.
- If a task reveals hidden complexity, split it before coding and keep test-first discipline.
- Use `@superpowers/systematic-debugging` if any regression appears in tool-runtime integration.
