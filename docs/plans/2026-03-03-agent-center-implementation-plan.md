# Agent-Center Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在现有 `agent-core` / `agent-session` / `agent-tool` 之上落地 `agent-center`，提供可生产启用的多 agent 控制面（`spawn_agent`/`wait`/`close_agent`），并满足深度/并发/幂等/恢复的鲁棒性要求。

**Architecture:** 新增 `agent-center` crate 作为控制平面层，维护 thread 生命周期、权限上下文、去重与持久化；运行执行仍委托给现有 `SessionRuntime`。通过 SQLite 存储 thread 与 dedup 状态，启动时执行 reconcile 对账，避免崩溃后状态漂移。工具层将 `spawn_agent`/`wait`/`close_agent` 注册到现有工具运行时，并由 `agent` facade/desktop 启动流程注入。

**Tech Stack:** Rust (workspace), Tokio, rusqlite, serde/toml, tracing, anyhow/thiserror, cargo test.

**Relevant Skills:** @test-driven-development @verification-before-completion @m07-concurrency @m12-lifecycle @m13-domain-error

---

## Preflight

### Task 0: 建立隔离开发环境

**Files:**
- Modify: `none`
- Test: `none`

**Step 1: 创建工作分支**

Run: `git checkout -b codex/agent-center-implementation`
Expected: 成功切换到新分支

**Step 2: 确认工作区干净（或记录脏文件）**

Run: `git status --short`
Expected: 仅出现预期变更；若有无关脏文件，先记录避免误提交

**Step 3: 提交空基线（可选）**

Run: `git commit --allow-empty -m "chore: start agent-center implementation"`
Expected: 生成基线提交（便于后续 review 分段）

---

### Task 1: 创建 `agent-center` crate 骨架并接入 workspace

**Files:**
- Create: `agent-center/Cargo.toml`
- Create: `agent-center/src/lib.rs`
- Create: `agent-center/src/error.rs`
- Create: `agent-center/tests/compile_smoke.rs`
- Modify: `Cargo.toml`
- Test: `agent-center/tests/compile_smoke.rs`

**Step 1: 先写失败测试（导出最小 API）**

```rust
// agent-center/tests/compile_smoke.rs
#[test]
fn exports_builder_api() {
    let _ = agent_center::AgentCenter::builder();
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center exports_builder_api`
Expected: FAIL（crate 不存在或 `AgentCenter` 未定义）

**Step 3: 写最小实现使测试通过**

```rust
// agent-center/src/lib.rs
pub mod error;

pub struct AgentCenter;
pub struct AgentCenterBuilder;

impl AgentCenter {
    pub fn builder() -> AgentCenterBuilder {
        AgentCenterBuilder
    }
}
```

**Step 4: 再跑测试确认通过**

Run: `cargo test -p agent-center exports_builder_api`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml agent-center/Cargo.toml agent-center/src/lib.rs agent-center/src/error.rs agent-center/tests/compile_smoke.rs
git commit -m "feat(agent-center): scaffold crate and workspace wiring"
```

---

### Task 2: 实现线程状态机与非法迁移保护

**Files:**
- Create: `agent-center/src/core/lifecycle.rs`
- Modify: `agent-center/src/lib.rs`
- Create: `agent-center/tests/lifecycle_state_machine_test.rs`
- Test: `agent-center/tests/lifecycle_state_machine_test.rs`

**Step 1: 写失败测试（合法/非法迁移）**

```rust
#[test]
fn rejects_terminal_state_regression() {
    use agent_center::core::lifecycle::{ThreadStateMachine, ThreadStatus};
    let mut sm = ThreadStateMachine::new(ThreadStatus::Running);
    sm.transition_to(ThreadStatus::Succeeded).unwrap();
    assert!(sm.transition_to(ThreadStatus::Running).is_err());
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center rejects_terminal_state_regression`
Expected: FAIL（状态机模块不存在或规则未实现）

**Step 3: 最小实现状态机**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStatus { Pending, Running, Succeeded, Failed, Cancelled, Closing, Closed }

pub struct ThreadStateMachine { status: ThreadStatus }

impl ThreadStateMachine {
    pub fn new(status: ThreadStatus) -> Self { Self { status } }
    pub fn status(&self) -> ThreadStatus { self.status }
    pub fn transition_to(&mut self, next: ThreadStatus) -> Result<(), &'static str> {
        let legal = matches!(
            (self.status, next),
            (ThreadStatus::Pending, ThreadStatus::Running)
                | (ThreadStatus::Running, ThreadStatus::Succeeded)
                | (ThreadStatus::Running, ThreadStatus::Failed)
                | (ThreadStatus::Running, ThreadStatus::Cancelled)
                | (ThreadStatus::Running, ThreadStatus::Closing)
                | (ThreadStatus::Closing, ThreadStatus::Closed)
                | (ThreadStatus::Closing, ThreadStatus::Failed)
        );
        if !legal { return Err("illegal transition"); }
        self.status = next;
        Ok(())
    }
}
```

**Step 4: 运行测试确认通过**

Run: `cargo test -p agent-center lifecycle_state_machine_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/core/lifecycle.rs agent-center/src/lib.rs agent-center/tests/lifecycle_state_machine_test.rs
git commit -m "feat(agent-center): add thread lifecycle state machine"
```

---

### Task 3: 实现深度/并发 Guard（RAII）并验证配额回收

**Files:**
- Create: `agent-center/src/permission/context.rs`
- Create: `agent-center/src/permission/guard.rs`
- Modify: `agent-center/src/lib.rs`
- Create: `agent-center/tests/guard_reservation_test.rs`
- Test: `agent-center/tests/guard_reservation_test.rs`

**Step 1: 写失败测试（超限拒绝 + drop 回收）**

```rust
#[tokio::test]
async fn releases_slot_on_drop() {
    use agent_center::permission::guard::SpawnGuards;
    let guards = SpawnGuards::new(1, 2);
    let r1 = guards.reserve(0).unwrap();
    assert!(guards.reserve(0).is_err());
    drop(r1);
    assert!(guards.reserve(0).is_ok());
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center releases_slot_on_drop`
Expected: FAIL（Guard 未实现）

**Step 3: 实现最小 Guard 与 Reservation**

```rust
pub struct SpawnGuards { /* max_concurrent, max_depth, running counter */ }
pub struct SpawnReservation { /* drop 时归还 */ }

impl SpawnGuards {
    pub fn new(max_concurrent: usize, max_depth: u32) -> Self { /* ... */ }
    pub fn reserve(&self, parent_depth: u32) -> Result<SpawnReservation, GuardError> { /* ... */ }
}

impl Drop for SpawnReservation {
    fn drop(&mut self) { /* release slot */ }
}
```

**Step 4: 运行测试确认通过**

Run: `cargo test -p agent-center guard_reservation_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/permission/context.rs agent-center/src/permission/guard.rs agent-center/src/lib.rs agent-center/tests/guard_reservation_test.rs
git commit -m "feat(agent-center): add depth and concurrency guards with RAII reservation"
```

---

### Task 4: 落地 SQLite schema 与线程存储仓库（含 dedup）

**Files:**
- Create: `agent-center/src/persistence/migrations.rs`
- Create: `agent-center/src/persistence/models.rs`
- Create: `agent-center/src/persistence/store.rs`
- Modify: `agent-center/src/lib.rs`
- Create: `agent-center/tests/sqlite_store_test.rs`
- Test: `agent-center/tests/sqlite_store_test.rs`

**Step 1: 写失败测试（同幂等键返回同线程）**

```rust
#[test]
fn dedup_returns_existing_thread_id() {
    // prepare sqlite store
    // insert spawn(parent=p1,key=k1,thread=t1)
    // repeat insert with same parent+key
    // assert still maps to t1
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center dedup_returns_existing_thread_id`
Expected: FAIL（schema 或 store 未实现）

**Step 3: 实现 migration + repository + 事务**

```rust
pub trait ThreadStore {
    fn upsert_thread(&self, thread: &ThreadRow) -> anyhow::Result<()>;
    fn get_by_dedup(&self, parent_thread_id: &str, key: &str) -> anyhow::Result<Option<String>>;
    fn insert_dedup(&self, parent_thread_id: &str, key: &str, thread_id: &str) -> anyhow::Result<()>;
}
```

**Step 4: 运行仓库测试**

Run: `cargo test -p agent-center sqlite_store_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/persistence/migrations.rs agent-center/src/persistence/models.rs agent-center/src/persistence/store.rs agent-center/src/lib.rs agent-center/tests/sqlite_store_test.rs
git commit -m "feat(agent-center): add sqlite persistence and spawn dedup table"
```

---

### Task 5: 实现配置加载与校验（`.agents/*.toml`）

**Files:**
- Create: `agent-center/src/config/mod.rs`
- Create: `agent-center/src/config/loader.rs`
- Create: `agent-center/src/config/validator.rs`
- Create: `agent-center/tests/config_loader_test.rs`
- Test: `agent-center/tests/config_loader_test.rs`

**Step 1: 写失败测试（坏配置不生效）**

```rust
#[test]
fn rejects_invalid_agent_config() {
    // load malformed toml
    // assert validation error contains field name
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center rejects_invalid_agent_config`
Expected: FAIL（loader/validator 未实现）

**Step 3: 实现 loader + validator**

```rust
pub struct AgentDefinition { /* name/version/prompt/tools/permissions/limits */ }
pub fn load_agents(dir: &Path) -> anyhow::Result<Vec<AgentDefinition>> { /* ... */ }
pub fn validate(def: &AgentDefinition) -> anyhow::Result<()> { /* ... */ }
```

**Step 4: 运行测试确认通过**

Run: `cargo test -p agent-center config_loader_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/config/mod.rs agent-center/src/config/loader.rs agent-center/src/config/validator.rs agent-center/tests/config_loader_test.rs
git commit -m "feat(agent-center): add agent definition loader and validator"
```

---

### Task 6: 实现 `spawn_agent` 内核（校验 + 幂等 + 持久化）

**Files:**
- Create: `agent-center/src/api/center.rs`
- Create: `agent-center/src/core/scheduler.rs`
- Create: `agent-center/src/core/registry.rs`
- Modify: `agent-center/src/lib.rs`
- Create: `agent-center/tests/spawn_flow_test.rs`
- Test: `agent-center/tests/spawn_flow_test.rs`

**Step 1: 写失败测试（重复幂等键只创建一次）**

```rust
#[tokio::test]
async fn spawn_is_idempotent_by_parent_and_key() {
    // first spawn -> thread t1
    // second spawn same parent+key -> still t1
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center spawn_is_idempotent_by_parent_and_key`
Expected: FAIL（spawn 流程未实现）

**Step 3: 最小实现 `AgentCenter::spawn`**

```rust
pub async fn spawn(&self, req: SpawnRequest) -> Result<SpawnResponse, AgentCenterError> {
    // validate agent + permission + guard
    // check dedup
    // create child session
    // persist thread + dedup in tx
    // dispatch initial input
}
```

**Step 4: 运行 spawn 测试**

Run: `cargo test -p agent-center spawn_flow_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/api/center.rs agent-center/src/core/scheduler.rs agent-center/src/core/registry.rs agent-center/src/lib.rs agent-center/tests/spawn_flow_test.rs
git commit -m "feat(agent-center): implement spawn flow with validation and idempotency"
```

---

### Task 7: 实现 `wait` 语义（any/all + timeout clamp + no busy loop）

**Files:**
- Create: `agent-center/src/tools/wait.rs`
- Modify: `agent-center/src/api/center.rs`
- Create: `agent-center/tests/wait_semantics_test.rs`
- Test: `agent-center/tests/wait_semantics_test.rs`

**Step 1: 写失败测试（`all` 模式超时行为）**

```rust
#[tokio::test]
async fn wait_all_times_out_when_any_thread_not_terminal() {
    // statuses: one running, one done
    // wait mode=all timeout=10
    // assert timed_out=true and status map returned
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center wait_all_times_out_when_any_thread_not_terminal`
Expected: FAIL

**Step 3: 实现 wait 核心逻辑**

```rust
pub async fn wait(&self, req: WaitRequest) -> Result<WaitResponse, AgentCenterError> {
    // clamp timeout to [1000, 300000]
    // subscribe and await condition; no spin polling
    // mode any/all
}
```

**Step 4: 运行 wait 测试**

Run: `cargo test -p agent-center wait_semantics_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/tools/wait.rs agent-center/src/api/center.rs agent-center/tests/wait_semantics_test.rs
git commit -m "feat(agent-center): implement wait semantics with timeout clamping"
```

---

### Task 8: 实现 `close_agent`（幂等关闭 + tombstone）

**Files:**
- Create: `agent-center/src/tools/close.rs`
- Modify: `agent-center/src/core/lifecycle.rs`
- Modify: `agent-center/src/api/center.rs`
- Create: `agent-center/tests/close_idempotency_test.rs`
- Test: `agent-center/tests/close_idempotency_test.rs`

**Step 1: 写失败测试（重复 close 返回一致快照）**

```rust
#[tokio::test]
async fn close_is_idempotent() {
    // close once -> closed snapshot
    // close again -> same terminal snapshot
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center close_is_idempotent`
Expected: FAIL

**Step 3: 实现 close 流程与 tombstone 记录**

```rust
pub async fn close(&self, req: CloseRequest) -> Result<CloseResponse, AgentCenterError> {
    // transition running->closing->closed
    // force path if requested
    // persist terminal snapshot/tombstone
}
```

**Step 4: 运行 close 测试**

Run: `cargo test -p agent-center close_idempotency_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/tools/close.rs agent-center/src/core/lifecycle.rs agent-center/src/api/center.rs agent-center/tests/close_idempotency_test.rs
git commit -m "feat(agent-center): implement idempotent close with tombstone state"
```

---

### Task 9: 实现启动对账 `reconcile()` 并接入初始化流程

**Files:**
- Create: `agent-center/src/core/reconciler.rs`
- Modify: `agent-center/src/api/center.rs`
- Modify: `agent-center/src/lib.rs`
- Create: `agent-center/tests/reconcile_test.rs`
- Test: `agent-center/tests/reconcile_test.rs`

**Step 1: 写失败测试（遗留 running 线程被修复）**

```rust
#[tokio::test]
async fn reconcile_marks_orphan_running_threads_terminal() {
    // seed running thread without active runtime
    // call reconcile
    // assert status changed to failed/closed with reason
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center reconcile_marks_orphan_running_threads_terminal`
Expected: FAIL

**Step 3: 实现对账逻辑**

```rust
pub async fn reconcile(&self) -> anyhow::Result<ReconcileReport> {
    // scan pending/running/closing
    // inspect runtime liveness
    // repair status + release guards
}
```

**Step 4: 运行对账测试**

Run: `cargo test -p agent-center reconcile_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-center/src/core/reconciler.rs agent-center/src/api/center.rs agent-center/src/lib.rs agent-center/tests/reconcile_test.rs
git commit -m "feat(agent-center): add startup reconcile and state repair"
```

---

### Task 10: 工具注册与上层集成（agent + desktop）

**Files:**
- Modify: `agent-tool/src/runtime.rs`
- Modify: `agent/src/builder.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `agent-center/src/tools/spawn.rs`
- Modify: `agent-center/src/tools/mod.rs`
- Create: `agent-center/tests/integration_tool_wiring_test.rs`
- Test: `agent-center/tests/integration_tool_wiring_test.rs`

**Step 1: 写失败测试（工具目录可见）**

```rust
#[tokio::test]
async fn runtime_lists_agent_center_tools() {
    // runtime.list_tools() contains spawn_agent/wait/close_agent
}
```

**Step 2: 运行测试确认失败**

Run: `cargo test -p agent-center runtime_lists_agent_center_tools`
Expected: FAIL

**Step 3: 实现注册与初始化接入**

```rust
// AgentCenter::register_builtin_tools(registry)
// register SpawnAgentTool, WaitTool, CloseAgentTool
```

**Step 4: 运行集成测试**

Run: `cargo test -p agent-center integration_tool_wiring_test`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/runtime.rs agent/src/builder.rs desktop/src-tauri/src/lib.rs agent-center/src/tools/spawn.rs agent-center/src/tools/mod.rs agent-center/tests/integration_tool_wiring_test.rs
git commit -m "feat(agent-center): wire built-in control tools into runtime and startup"
```

---

### Task 11: 完成验证、文档与回归测试

**Files:**
- Modify: `docs/plans/2026-03-03-agent-center-design.md`
- Create: `docs/agent-center-runbook.md`
- Test: `agent-center/tests/*`, 相关 workspace 回归

**Step 1: 写失败测试（端到端主链路）**

```rust
#[tokio::test]
async fn e2e_spawn_wait_close_flow() {
    // spawn child
    // wait(any)
    // close child
    // assert terminal and persisted
}
```

**Step 2: 跑端到端测试确认失败**

Run: `cargo test -p agent-center e2e_spawn_wait_close_flow`
Expected: FAIL

**Step 3: 最小补齐遗漏实现与文档**

```text
- 修复 e2e 暴露出的状态/持久化/超时边界问题
- 在 runbook 记录故障恢复、对账、指标说明
```

**Step 4: 全量验证**

Run: `cargo test -p agent-center`
Expected: PASS  

Run: `cargo test -p agent -p agent-session -p agent-tool`
Expected: PASS（无回归）

**Step 5: Commit**

```bash
git add docs/plans/2026-03-03-agent-center-design.md docs/agent-center-runbook.md agent-center
git commit -m "feat(agent-center): finalize implementation with e2e validation and runbook"
```

---

## Final Verification Checklist

- `spawn_agent` 在无 Guard/幂等/对账时不可启用（feature gate 或 hard check）
- `wait` 最小超时钳制生效，无 busy loop
- `close_agent` 幂等且有 tombstone
- 崩溃重启后 `reconcile()` 幂等可重复执行
- 文档、测试、代码实现一致

## Suggested PR Breakdown

1. PR-1: crate skeleton + lifecycle + guards
2. PR-2: sqlite persistence + config loader/validator
3. PR-3: spawn/wait/close core flow
4. PR-4: reconcile + integration wiring + docs/runbook

