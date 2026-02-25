# Prompt Lab SQL-First Rewrite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在不做兼容迁移的前提下，按 SQL-First 设计重写 Prompt Lab 的数据库契约、核心 domain、服务语义、Tauri 命令契约和前端 API 类型。

**Architecture:** 先锁数据库契约与测试，再重写 `prompt_lab_core` 的 domain/repository/service，随后重写 Tauri DTO/commands，最后对齐前端 API。`sop_steps` 是真源，`SopAggregate` 负责聚合阶段步骤；`check_results` 在服务层执行 AI/Manual 分流写入规则。

**Tech Stack:** Rust (`sqlx`, `tokio`, `serde`), SQLite, Tauri v2, TypeScript

---

## Preflight

1. 在独立 worktree 执行本计划（避免污染当前工作目录）。
2. 每个任务完成后立即运行对应测试并提交。
3. 技能引用：`@test-driven-development` `@m09-domain` `@verification-before-completion`。

### Task 1: 锁定 SQL 契约与迁移文件

**Files:**
- Create: `prompt_lab_core/migrations/202602250001_init_prompt_lab_sqlite_v2.up.sql`
- Create: `prompt_lab_core/migrations/202602250001_init_prompt_lab_sqlite_v2.down.sql`
- Modify: `prompt_lab_core/migrations/`（移除旧 migration）
- Create: `prompt_lab_core/tests/sql_schema_v2.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn check_results_manual_unique_index_contract() {
    let lab = test_lab().await;
    let pool = lab.pool();
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT sql FROM sqlite_master WHERE type='index' AND name='idx_check_results_manual_latest'"
    ).fetch_all(pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].0.contains("WHERE source_type = 2 AND check_item_id IS NOT NULL"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p prompt_lab_core --test sql_schema_v2 check_results_manual_unique_index_contract -- --nocapture`  
Expected: FAIL（索引不存在或 SQL 不匹配）

**Step 3: Write minimal implementation**

```sql
CREATE UNIQUE INDEX idx_check_results_manual_latest
ON check_results (context_type, context_key, check_item_id)
WHERE source_type = 2 AND check_item_id IS NOT NULL;
```

同时在新 migration 里定义完整 v2 DDL，并删除旧迁移文件，保证新库首次启动即为目标结构。

**Step 4: Run test to verify it passes**

Run: `cargo test -p prompt_lab_core --test sql_schema_v2 -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add prompt_lab_core/migrations prompt_lab_core/tests/sql_schema_v2.rs
git commit -m "feat(prompt_lab_core): define SQL-first v2 schema"
```

### Task 2: 重写 Domain 类型定义

**Files:**
- Modify: `prompt_lab_core/src/domain.rs`
- Create: `prompt_lab_core/tests/domain_contract_v2.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn check_result_allows_nullable_check_item_id() {
    let v = serde_json::json!({
      "id": 1,
      "context_type": "sop",
      "context_key": "sop:SOP-1",
      "check_item_id": null,
      "source_type": "manual",
      "operator_id": "u1",
      "result": {"ok": true},
      "is_pass": true,
      "created_at": 1730000000000i64
    });
    let parsed: CheckResult = serde_json::from_value(v).unwrap();
    assert!(parsed.check_item_id.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p prompt_lab_core --test domain_contract_v2 -- --nocapture`  
Expected: FAIL（字段类型或枚举不匹配）

**Step 3: Write minimal implementation**

```rust
pub struct CheckResult {
    pub context_key: String,
    pub check_item_id: Option<i64>,
    // ...
}
```

补齐：
- `ChecklistContextType` 枚举；
- `SopStepRef` 与 `SopAggregate`；
- `context_key` 全量替代数值型 context id。

**Step 4: Run test to verify it passes**

Run: `cargo test -p prompt_lab_core --test domain_contract_v2 -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add prompt_lab_core/src/domain.rs prompt_lab_core/tests/domain_contract_v2.rs
git commit -m "feat(prompt_lab_core): redefine v2 domain contracts"
```

### Task 3: 实现 Repository（check_results 分流写入）

**Files:**
- Modify: `prompt_lab_core/src/repository.rs`
- Modify: `prompt_lab_core/tests/core_flow.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn manual_with_non_null_check_item_keeps_single_latest() {
    let lab = test_lab().await;
    let first = lab.check_result_service().upsert_or_append(input_manual(Some(7), false)).await.unwrap();
    let second = lab.check_result_service().upsert_or_append(input_manual(Some(7), true)).await.unwrap();
    assert_eq!(first.id, second.id);
    let listed = lab.check_result_service().list(filter_key()).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert!(listed[0].is_pass);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p prompt_lab_core core_flow::manual_with_non_null_check_item_keeps_single_latest -- --nocapture`  
Expected: FAIL（当前语义为直接插入或不支持 nullable）

**Step 3: Write minimal implementation**

```rust
if input.source_type == SourceType::Manual && input.check_item_id.is_some() {
    // update by (context_type, context_key, check_item_id, source_type=manual), fallback insert
} else {
    // insert
}
```

并把所有 SQL 查询条件切换为 `context_key`。

**Step 4: Run test to verify it passes**

Run: `cargo test -p prompt_lab_core --test core_flow -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add prompt_lab_core/src/repository.rs prompt_lab_core/tests/core_flow.rs
git commit -m "feat(prompt_lab_core): implement v2 check result write semantics"
```

### Task 4: 实现 SOP 聚合与快照修正规则

**Files:**
- Modify: `prompt_lab_core/src/repository.rs`
- Modify: `prompt_lab_core/src/service.rs`
- Modify: `prompt_lab_core/tests/core_flow.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn get_sop_returns_aggregate_and_normalizes_snapshot_names() {
    let lab = test_lab().await;
    let step = create_step_named(&lab, "真实名称").await;
    create_sop_with_detect_refs(&lab, vec![json!({"sop_step_id": step.id, "name": "旧名称"})]).await;
    let agg = lab.sop_service().get_sop_aggregate_by_sop_id("SOP-1").await.unwrap();
    assert_eq!(agg.detect_steps[0].name, "真实名称");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p prompt_lab_core core_flow::get_sop_returns_aggregate_and_normalizes_snapshot_names -- --nocapture`  
Expected: FAIL（没有聚合接口或名称未归一）

**Step 3: Write minimal implementation**

```rust
pub async fn get_sop_aggregate_by_sop_id(&self, sop_id: &str) -> Result<SopAggregate> {
    // read sop -> parse step refs -> batch load sop_steps -> compose ordered stage arrays
}
```

并在写入 SOP 时校验：
- `sop_step_id` 必须存在；
- `sop_step_id` 必须属于当前 `sop_id`。

**Step 4: Run test to verify it passes**

Run: `cargo test -p prompt_lab_core --test core_flow -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add prompt_lab_core/src/repository.rs prompt_lab_core/src/service.rs prompt_lab_core/tests/core_flow.rs
git commit -m "feat(prompt_lab_core): add sop aggregate and step-ref validation"
```

### Task 5: 对齐 prompt_lab_core 公共导出与构造器

**Files:**
- Modify: `prompt_lab_core/src/lib.rs`
- Modify: `prompt_lab_core/src/error.rs`
- Modify: `prompt_lab_core/src/service.rs`
- Test: `prompt_lab_core/tests/core_flow.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn manual_without_is_pass_defaults_to_true() {
    let lab = test_lab().await;
    let r = lab.check_result_service().upsert_or_append(input_manual_default_pass()).await.unwrap();
    assert!(r.is_pass);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p prompt_lab_core core_flow::manual_without_is_pass_defaults_to_true -- --nocapture`  
Expected: FAIL（默认值未生效）

**Step 3: Write minimal implementation**

```rust
let is_pass = match input.source_type {
    SourceType::Manual => input.is_pass.unwrap_or(true),
    SourceType::Ai => input.is_pass.unwrap_or(false),
};
```

并统一错误码映射：`INVALID_INPUT/NOT_FOUND/CONFLICT/DB_ERROR/PARSE_ERROR`。

**Step 4: Run test to verify it passes**

Run: `cargo test -p prompt_lab_core --test core_flow -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add prompt_lab_core/src/lib.rs prompt_lab_core/src/service.rs prompt_lab_core/src/error.rs prompt_lab_core/tests/core_flow.rs
git commit -m "feat(prompt_lab_core): enforce manual default pass semantics"
```

### Task 6: 重写 Tauri DTO 与命令层

**Files:**
- Modify: `argusx-desktop/src-tauri/src/lib.rs`
- Test: `cargo check` for `argusx-desktop`

**Step 1: Write the failing test/compile target**

```bash
cargo check -p argusx-desktop
```

Expected: FAIL（DTO 字段与新 domain 不匹配）

**Step 2: Run to verify it fails**

Run: `cargo check -p argusx-desktop`  
Expected: compile errors around `context_id`, `check_item_id`, or enum mapping

**Step 3: Write minimal implementation**

```rust
#[derive(Deserialize)]
struct UpsertOrAppendCheckResultInput {
    context_type: String,
    context_key: String,
    check_item_id: Option<i64>,
    source_type: SourceTypeDto,
    is_pass: Option<bool>,
    // ...
}
```

并新增/调整命令：
- `upsert_or_append_check_result`
- `get_sop` 返回 `SopAggregateDto`
- `list_ai_execution_logs` 按 `context_key` 过滤

**Step 4: Run check to verify it passes**

Run: `cargo check -p argusx-desktop`  
Expected: PASS

**Step 5: Commit**

```bash
git add argusx-desktop/src-tauri/src/lib.rs
git commit -m "feat(tauri): align prompt-lab command DTOs with v2 domain"
```

### Task 7: 对齐前端 API 契约与 mock

**Files:**
- Modify: `argusx-desktop/lib/api/prompt-lab.ts`
- Modify: `argusx-desktop/lib/mocks/prompt-lab-mock.ts`
- Modify: `argusx-desktop/lib/mocks/data.ts`

**Step 1: Write failing type check**

Run: `pnpm --dir argusx-desktop exec tsc --noEmit`  
Expected: FAIL（`context_id` 等旧字段未对齐）

**Step 2: Run to verify it fails**

Run: `pnpm --dir argusx-desktop exec tsc --noEmit`  
Expected: type errors in prompt-lab API and call sites

**Step 3: Write minimal implementation**

```ts
export interface CheckResult {
  context_type: string;
  context_key: string;
  check_item_id: number | null;
  source_type: "ai" | "manual";
  is_pass: boolean;
}
```

同步更新 mock 的过滤键、新增命令名和 `is_pass` 默认逻辑。

**Step 4: Run check to verify it passes**

Run: `pnpm --dir argusx-desktop exec tsc --noEmit`  
Expected: PASS

**Step 5: Commit**

```bash
git add argusx-desktop/lib/api/prompt-lab.ts argusx-desktop/lib/mocks/prompt-lab-mock.ts argusx-desktop/lib/mocks/data.ts
git commit -m "feat(frontend): sync prompt-lab API types with v2 backend contract"
```

### Task 8: 全链路验证与文档收尾

**Files:**
- Modify: `docs/plans/2026-02-25-prompt-lab-redesign-design.md`（如有偏差，修正文档）
- Create: `docs/plans/2026-02-25-prompt-lab-redesign-verification.md`

**Step 1: Write verification checklist**

```md
- [ ] SQL schema tests pass
- [ ] prompt_lab_core tests pass
- [ ] argusx-desktop cargo check pass
- [ ] frontend tsc pass
```

**Step 2: Run full verification**

Run:
```bash
cargo test -p prompt_lab_core
cargo check -p argusx-desktop
pnpm --dir argusx-desktop exec tsc --noEmit
```
Expected: all PASS

**Step 3: Record outputs**

把命令结果摘要写入 `docs/plans/2026-02-25-prompt-lab-redesign-verification.md`。

**Step 4: Final sanity check**

Run: `git status --short`  
Expected: only expected implementation files changed

**Step 5: Commit**

```bash
git add docs/plans/2026-02-25-prompt-lab-redesign-verification.md docs/plans/2026-02-25-prompt-lab-redesign-design.md
git commit -m "docs(prompt-lab): add rewrite verification report"
```

