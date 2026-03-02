# Agent Turn 后置校验生命周期设计（含 Desktop 集成）

## 1. 背景与目标

当前 `agent-turn` 已采用事件驱动状态机（`TurnState -> RuntimeEvent -> Transition(Effect)`）闭环。现需在 turn 完成前增加“后验师”阶段，用于：

1. 执行硬编码后置校验 Tool。
2. 校验输出可入库（退出码 + JSON 协议）。
3. 校验失败时自动唤醒主 Agent 自修复。
4. 全流程保持在同一 turn 内，保留现有上下文、token 统计、UI 事件流与取消语义。

## 2. 非目标

1. 不改造成外层 runtime 包裹式二段流程。
2. 不要求每次后置失败都人工确认。
3. 不在本次引入新持久化配置系统（仅支持运行时可选启用）。

## 3. 已确认决策

1. 后置校验失败自动重试最多 `3` 轮。
2. 校验协议使用“退出码 + JSON”。
3. 成功收口时 `final_message` 优先使用 validator 返回 `summary`，缺失则回退 Agent 原始 `output_buffer`。
4. 校验失败回环时同时注入 `system_note + user_input`。
5. Desktop 端需可观测后置阶段。
6. 后验师为可选能力：未配置相关 tool 时，跳过后置阶段并直接按原逻辑完成 turn。

## 4. 方案对比

### 方案 A（采用）：新增 PostProcessing 生命周期

- 扩展 `Lifecycle/RuntimeEvent/Effect`。
- 在 `maybe_finalize` 拦截 `Done`，先进入 `PostProcessing`。
- 成功再 `Done`；失败则回 `Active` 重试或最终 `Failed`。

优点：

1. 与当前 Event-Sourcing 架构一致，语义最清晰。
2. 取消、幂等、重放与现有 reducer 范式一致。
3. 可自然扩展 UI 与 Run 可观测事件。

缺点：

1. 需要改 enum 与序列化分支，改动跨 crate。

### 方案 B：复用普通 Tool 事件并约定特殊 tool_name

优点：改动面较小。  
缺点：后置阶段语义被混入普通工具流，后续维护与测试成本高。

### 方案 C：外层 runtime 包裹后处理

优点：核心 reducer 改动少。  
缺点：破坏单环路语义，取消/状态一致性差，不符合现有架构优势。

## 5. 推荐架构

```text
Active (model/tool loop complete)
  -> [if post-validator disabled] Done
  -> [if enabled] PostProcessing
       -> PostValidatorSuccess -> Done
       -> PostValidatorFailed(attempt < 3) -> Active (epoch+1, inject fix prompt)
       -> PostValidatorFailed(attempt >= 3) -> Failed
```

`TurnEngine` 主循环退出条件保持不变：仅 `Done | Failed` 退出，因此 `PostProcessing` 会自然留在同一 turn 内。

## 6. 数据模型设计

### 6.1 `agent-turn` 状态与配置

`agent-turn/src/state.rs`

1. `Lifecycle` 新增：
   - `PostProcessing`
2. `TurnState` 新增：
   - `post_validate_attempt: u8`
3. `TurnEngineConfig` 新增可选配置：
   - `post_validator: Option<PostValidatorConfig>`
4. `PostValidatorConfig` 建议字段：
   - `enabled: bool`（默认 `true`，当对象存在时）
   - `max_attempts: u8`（默认 `3`）
   - `tool_name: String`（默认例如 `post_validator`，硬编码通道标识）

约束：

1. `post_validator = None` 视为功能关闭，保持现有完成路径。
2. `Some(config)` 才进入 `PostProcessing`。

### 6.2 `agent-turn` Effect

`agent-turn/src/effect.rs`

新增：

- `Effect::ExecutePostValidator { turn_id, summary, attempt }`

### 6.3 `agent-core` RuntimeEvent

`agent-core/src/runtime_event.rs`

新增：

1. `PostValidatorSuccess { event_id, summary: Option<String> }`
2. `PostValidatorFailed { event_id, error_message: String }`

并同步：

1. `id()` 分支。
2. `with_new_id()` 分支。
3. serde 序列化兼容（snake_case tag）。

### 6.4 可观测事件（建议）

`agent-core/src/events.rs`

建议新增：

1. `RunStreamEvent::PostValidationStarted { turn_id, attempt }`
2. `RunStreamEvent::PostValidationFailed { turn_id, attempt, can_retry, message }`
3. `UiThreadEvent::PostValidationStarted { turn_id, attempt }`
4. `UiThreadEvent::PostValidationFailed { turn_id, attempt, can_retry, message }`

说明：若需控制改动面，可先只加 `RunStreamEvent`，前端读取 run 事件即可。

## 7. Reducer 行为设计

`agent-turn/src/reducer.rs`

### 7.1 `maybe_finalize` 改造

现状：`can_finish()` 后直接 `Done`。  
目标：按配置条件分流。

流程：

1. 若 `!can_finish()`，返回。
2. 若 `post_validator` 未启用（`None`），走当前原始 `Done` 收口逻辑。
3. 若启用：
   - 置 `lifecycle = PostProcessing`
   - `post_validate_attempt += 1`
   - 发 `Effect::ExecutePostValidator`
   - 发 `PostValidationStarted`（可选）
   - 不发 `TurnDone/Ui::Done`

### 7.2 处理 `PostValidatorSuccess`

Guard：仅 `lifecycle == PostProcessing` 时处理。

行为：

1. `lifecycle = Done`
2. `done_emitted = true`
3. `final_message = validator.summary.or(output_buffer)`
4. 发 `TurnDone` / `Ui::Done`
5. `PersistCheckpoint`

### 7.3 处理 `PostValidatorFailed`

Guard：仅 `lifecycle == PostProcessing` 时处理。

行为：

1. 读取 `attempt` 与 `max_attempts`。
2. 若 `attempt < max_attempts`：
   - `lifecycle = Active`
   - `model_state = Streaming`
   - `epoch += 1`
   - `transcript` 追加：
     - `system_note(Warning, validator_error)`
     - `InputEnvelope::user_text(fix_prompt)`
   - `start_model_from_pending(next_epoch)`
3. 若 `attempt >= max_attempts`：
   - `fail_turn(message, cancelled=false)`

其中 `fix_prompt` 固定模板：

- 明确失败原因。
- 要求修复并重新输出最终 summary。

### 7.4 取消语义

`CancelRequested` 行为不变。在 `PostProcessing` 阶段同样可终止 turn，最终 `TurnFailed(cancelled=true)`。

## 8. 后验师执行与协议

### 8.1 执行位置

在 `EffectExecutor::execute()` 增加 `ExecutePostValidator` 分支，执行硬编码 tool 调用（非模型 tool_call）。

### 8.2 协议（退出码 + JSON）

成功需同时满足：

1. `exit_code == 0`
2. stdout 可解析为 JSON
3. JSON `ok == true`

建议 JSON：

1. 成功：`{ "ok": true, "summary": "..." }`
2. 失败：`{ "ok": false, "error": "..." }`

失败触发条件（任一满足）：

1. 非零退出码
2. stdout 非法 JSON
3. `ok != true`

失败映射到 `RuntimeEvent::PostValidatorFailed { error_message }`。

### 8.3 可选能力语义

1. **未配置**后验师（`post_validator = None`）：直接跳过。
2. **已配置但运行异常**（例如执行失败/协议错误）：视为校验失败，进入自动修复流程。
3. 不引入 silent skip（避免误以为已校验成功）。

## 9. Desktop 集成设计

### 9.1 Tauri 事件桥接

`desktop/src-tauri/src/lib.rs` 现有 `spawn_stream_forwarders` 已透传 run/ui 事件 JSON；新增事件无需额外桥接逻辑。

### 9.2 前端 Store 映射

`desktop/lib/stores/chat-store.ts`

新增事件消费：

1. `post_validation_started`：
   - turn 维持 `streaming`
   - session 设为 `thinking`（V1 复用状态）
2. `post_validation_failed`（可重试）：
   - turn 写入 warning/临时错误文案
   - session 保持 `thinking`

最终收口保持：

1. `turn_done` -> `await-input`
2. `turn_failed` -> `wait-input`

### 9.3 UI 展示

V1 推荐：不新增 `ChatStatus` 枚举，复用 `thinking`，在 turn 卡片显示：

- `Validating output (attempt x/3)`

V2（后续可选）：新增独立 `validating` badge。

### 9.4 用户交互策略

后置重试全自动，不要求每轮人工确认；仅在最终失败时用户介入。

## 10. 兼容性与幂等

1. `reduce` 开头的 `seen_event_ids` 去重机制保持不变，保障重复事件不二次收口。
2. `Done/Failed` 早返回机制保持不变。
3. `PostProcessing` 仅引入新中间态，不改变既有 `Active/Backoff` 行为。

## 11. 测试设计

### 11.1 Reducer 单测新增

1. `can_finish + validator disabled -> 直接 Done`
2. `can_finish + validator enabled -> PostProcessing（非 Done）`
3. `PostValidatorSuccess -> Done`
4. `PostValidatorSuccess` 的 `final_message` 优先 validator summary
5. `PostValidatorFailed (attempt<3) -> Active + epoch+1 + 注入修复输入`
6. `PostValidatorFailed (attempt>=3) -> Failed`
7. `CancelRequested in PostProcessing -> Failed(cancelled=true)`
8. 重放幂等：重复 `PostValidatorSuccess/Failed` 不重复发终态事件

### 11.2 Effect 执行单测新增

1. 退出码成功 + 合法 JSON -> `PostValidatorSuccess`
2. 非零退出码 -> `PostValidatorFailed`
3. 非法 JSON -> `PostValidatorFailed`
4. `ok=false` -> `PostValidatorFailed`

### 11.3 Desktop 前端测试新增

1. 收到 `post_validation_started` 时 UI 显示 validating 文案。
2. 收到可重试失败事件时保持 streaming 态，不提前 done/failed。
3. `turn_done/turn_failed` 仍正确收口 session 状态。

## 12. 风险与缓解

1. 风险：后验师协议波动导致误判。  
   缓解：统一协议解析与明确错误文案，失败入可观测事件。
2. 风险：无限回环。  
   缓解：硬上限 `max_attempts=3`。
3. 风险：前端无新状态导致用户感知不足。  
   缓解：至少输出 validating 文案与尝试次数。

## 13. 交付物

1. `PostProcessing` 生命周期与可选启用逻辑。
2. 后置校验 effect 与 runtime event。
3. 自动修复回环（最多 3 次）与最终失败收口。
4. Desktop 对后置阶段的事件可视化。
5. 覆盖新增状态路径的测试。

---

Created: 2026-03-02  
Status: Approved for implementation planning
