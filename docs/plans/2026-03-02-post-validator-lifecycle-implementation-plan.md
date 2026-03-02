# Post-Validator Lifecycle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在 `agent-turn` 中引入可选的后置校验生命周期（PostProcessing），支持自动修复回环（最多 3 次），并把状态正确透传到 Desktop UI。

**Architecture:** 以状态机原生扩展为主：`maybe_finalize` 按配置分流，启用后先进入 `PostProcessing` 并触发 `ExecutePostValidator`。校验成功再 `Done`，失败则自动注入修复提示并重启模型循环，超限后 `TurnFailed`。Desktop 继续复用 Tauri 事件透传，仅在前端 store 增加新事件消费与展示文案。

**Tech Stack:** Rust (`agent-core`, `agent-turn`), Tokio, serde, Tauri, Next.js + Zustand, pnpm.

---

## Execution Notes

1. 开发过程遵循 `@test-driven-development`（先写失败测试，再最小实现）。
2. 失败排查使用 `@systematic-debugging`（不要盲改）。
3. 完成前执行 `@verification-before-completion`（用命令输出作为结论依据）。
4. 保持 DRY / YAGNI：仅引入设计文档已确认范围内的字段和事件。

### Task 1: Extend Core Event Contracts (`agent-core`)

**Files:**
- Modify: `agent-core/src/runtime_event.rs`
- Modify: `agent-core/src/events.rs`
- Modify: `agent-core/src/lib.rs`
- Test: `agent-core/src/lib.rs`

**Step 1: Write the failing test**

在 `agent-core/src/lib.rs` 的测试模块新增：

```rust
#[test]
fn runtime_event_roundtrip_post_validator_json() {
    let ev = RuntimeEvent::PostValidatorSuccess {
        event_id: new_id(),
        summary: Some("clean summary".to_string()),
    };
    let raw = serde_json::to_string(&ev).expect("serialize runtime event");
    let got: RuntimeEvent = serde_json::from_str(&raw).expect("deserialize runtime event");
    assert_eq!(ev, got);
}
```

并新增 `RunStreamEvent::PostValidationStarted` / `UiThreadEvent::PostValidationStarted` 的序列化 smoke test。

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core runtime_event_roundtrip_post_validator_json -- --exact`  
Expected: FAIL，报错 `no variant named PostValidatorSuccess`（或等价 enum variant 缺失错误）。

**Step 3: Write minimal implementation**

实现最小闭环：

```rust
pub enum RuntimeEvent {
    // ...
    PostValidatorSuccess { event_id: Id, summary: Option<String> },
    PostValidatorFailed { event_id: Id, error_message: String },
}
```

并同步更新：

1. `RuntimeEvent::id()` 匹配分支。
2. `RuntimeEvent::with_new_id()` 匹配分支。
3. `RunStreamEvent` 与 `UiThreadEvent` 新增 post-validation 事件。

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-core`  
Expected: PASS，`agent-core` 全部测试通过。

**Step 5: Commit**

```bash
git add agent-core/src/runtime_event.rs agent-core/src/events.rs agent-core/src/lib.rs
git commit -m "feat(agent-core): add post-validator runtime and stream events"
```

### Task 2: Add Optional Post-Validator Config and Finalize Intercept Scaffolding

**Files:**
- Modify: `agent-turn/src/state.rs`
- Modify: `agent-turn/src/effect.rs`
- Modify: `agent-turn/src/reducer.rs`
- Modify: `agent-turn/src/test_helpers.rs`
- Modify: `agent-turn/src/lib.rs`
- Test: `agent-turn/src/reducer.rs`

**Step 1: Write the failing test**

在 `agent-turn/src/reducer.rs` 的测试模块新增两个用例：

1. `model_completed_without_post_validator_finishes_directly`
2. `model_completed_with_post_validator_enters_postprocessing`

关键断言：

```rust
assert_eq!(result.state.lifecycle, Lifecycle::PostProcessing);
assert!(result.effects.iter().any(|e| matches!(e, Effect::ExecutePostValidator { .. })));
assert!(!result.run_events.iter().any(|e| matches!(e, RunStreamEvent::TurnDone { .. })));
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn model_completed_with_post_validator_enters_postprocessing -- --exact`  
Expected: FAIL，报错 `Lifecycle::PostProcessing` 或 `Effect::ExecutePostValidator` 不存在。

**Step 3: Write minimal implementation**

新增状态与配置脚手架：

```rust
pub enum Lifecycle { Active, Backoff, PostProcessing, Done, Failed }

pub struct PostValidatorConfig {
    pub tool_name: String,
    pub max_attempts: u8,
}

pub struct TurnEngineConfig {
    pub max_parallel_tools: usize,
    pub retry_policy: RetryPolicy,
    pub post_validator: Option<PostValidatorConfig>,
}
```

新增 effect：

```rust
Effect::ExecutePostValidator { turn_id: String, summary: String, attempt: u8, tool_name: String }
```

改造 `maybe_finalize`：当 `post_validator == None` 走旧逻辑；`Some(_)` 时进入 `PostProcessing` 并发新 effect。

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn model_completed_without_post_validator_finishes_directly -- --exact`  
Run: `cargo test -p agent-turn model_completed_with_post_validator_enters_postprocessing -- --exact`  
Expected: PASS。

**Step 5: Commit**

```bash
git add agent-turn/src/state.rs agent-turn/src/effect.rs agent-turn/src/reducer.rs agent-turn/src/test_helpers.rs agent-turn/src/lib.rs
git commit -m "feat(agent-turn): add optional post-validator lifecycle scaffold"
```

### Task 3: Implement Reducer Success/Failure Loop for Post-Validation

**Files:**
- Modify: `agent-turn/src/reducer.rs`
- Modify: `agent-turn/src/test_helpers.rs`
- Test: `agent-turn/src/reducer.rs`

**Step 1: Write the failing test**

新增 reducer 用例（至少）：

1. `post_validator_success_emits_turn_done_with_summary_precedence`
2. `post_validator_failed_requeues_fix_prompt_and_restarts_model`
3. `post_validator_failed_hits_max_attempts_and_fails_turn`
4. `cancel_requested_in_postprocessing_fails_turn`

示例断言：

```rust
assert_eq!(result.state.lifecycle, Lifecycle::Done);
assert!(result.run_events.iter().any(|e| matches!(e, RunStreamEvent::TurnDone { final_message: Some(msg), .. } if msg == "validator summary")));
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn post_validator_success_emits_turn_done_with_summary_precedence -- --exact`  
Expected: FAIL，`RuntimeEvent::PostValidatorSuccess` 未处理或状态不匹配。

**Step 3: Write minimal implementation**

在 `reduce()` 中加入：

1. `RuntimeEvent::PostValidatorSuccess` 分支：
   - `PostProcessing -> Done`
   - `final_message = validator.summary.or(output_buffer)`
   - 发 `TurnDone/Ui::Done/PersistCheckpoint`
2. `RuntimeEvent::PostValidatorFailed` 分支：
   - `attempt < max_attempts`：回 `Active`，`epoch+1`，注入 `system_note + user_text`，`start_model_from_pending`
   - 否则：`fail_turn(...)`

并发 `RunStreamEvent::PostValidationFailed`（可重试与否）。

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn post_validator_`  
Expected: PASS，新增 `post_validator_*` 测试全部通过。

**Step 5: Commit**

```bash
git add agent-turn/src/reducer.rs agent-turn/src/test_helpers.rs
git commit -m "feat(agent-turn): implement post-validator success/failure retry loop"
```

### Task 4: Implement EffectExecutor Post-Validator Protocol Mapping

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Test: `agent-turn/src/effect.rs`

**Step 1: Write the failing test**

在 `agent-turn/src/effect.rs` 测试模块新增：

1. `post_validator_effect_emits_success_event_on_ok_json`
2. `post_validator_effect_emits_failed_event_on_nonzero_exit`
3. `post_validator_effect_emits_failed_event_on_invalid_json`

建议构造工具返回：

```json
{"ok": true, "summary": "normalized"}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn post_validator_effect_emits_success_event_on_ok_json -- --exact`  
Expected: FAIL，`Effect::ExecutePostValidator` 分支不存在或未派发 `PostValidatorSuccess`。

**Step 3: Write minimal implementation**

在 `EffectExecutor::execute()` 增加分支：

1. 调用硬编码 validator tool（基于 `tool_name`）。
2. 校验 `exit_code`。
3. 解析 stdout JSON，判定 `ok`。
4. 发送：
   - `RuntimeEvent::PostValidatorSuccess { summary }`
   - 或 `RuntimeEvent::PostValidatorFailed { error_message }`

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn post_validator_effect_`  
Expected: PASS，新增 effect 测试全部通过。

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs
git commit -m "feat(agent-turn): execute post-validator effect and map protocol results"
```

### Task 5: Wire Desktop Event Consumption and Validating UX Hint

**Files:**
- Modify: `desktop/lib/stores/chat-store.ts`
- Modify: `desktop/components/features/chat/agent-turn-card.tsx`
- (Optional) Modify: `desktop/components/features/chat/session-badge.tsx`

**Step 1: Implement event handling branch**

在 `applyAgentStreamEnvelope` 增加对以下事件处理：

1. `post_validation_started`：session 设为 `thinking`，turn 维持 `streaming`
2. `post_validation_failed`：更新 turn warning/error 字段（可重试时不终止 turn）

在 turn 卡片展示文案：

```ts
Validating output (attempt x/3)
```

**Step 2: Run static verification**

Run: `pnpm --dir desktop lint`  
Expected: PASS。

**Step 3: Run type verification**

Run: `pnpm --dir desktop exec tsc --noEmit`  
Expected: PASS。

**Step 4: Commit**

```bash
git add desktop/lib/stores/chat-store.ts desktop/components/features/chat/agent-turn-card.tsx desktop/components/features/chat/session-badge.tsx
git commit -m "feat(desktop): surface post-validation streaming and retry hints"
```

### Task 6: End-to-End Verification and Final Integration Commit

**Files:**
- Modify: `agent-turn/src/reducer.rs` (if final polish needed)
- Modify: `agent-turn/src/effect.rs` (if final polish needed)
- Modify: `desktop/lib/stores/chat-store.ts` (if final polish needed)
- Modify: `docs/plans/2026-03-02-post-validator-lifecycle-design.md` (if behavior notes need sync)

**Step 1: Run focused Rust tests**

Run:

```bash
cargo test -p agent-core
cargo test -p agent-turn
```

Expected: PASS。

**Step 2: Run desktop checks**

Run:

```bash
pnpm --dir desktop lint
pnpm --dir desktop exec tsc --noEmit
```

Expected: PASS。

**Step 3: Manual runtime verification**

1. 配置 `post_validator = None`，确认 turn 直接 `Done`。
2. 配置 validator 并返回 `ok=true`，确认 `PostProcessing -> Done`。
3. 返回 `ok=false` 连续 3 次，确认最终 `TurnFailed`。
4. 在 `PostProcessing` 点击取消，确认 `cancelled=true`。

**Step 4: Final commit**

```bash
git add agent-core agent-turn desktop docs/plans/2026-03-02-post-validator-lifecycle-design.md
git commit -m "feat(agent-turn): add optional post-validator lifecycle with desktop visibility"
```

## Verification Gate (Before Merge)

1. `cargo test -p agent-core && cargo test -p agent-turn`
2. `pnpm --dir desktop lint && pnpm --dir desktop exec tsc --noEmit`
3. 确认无重复 `TurnDone` / `TurnFailed` 终态事件。
4. 确认未配置后验师时行为与旧版本一致。

