# `update_plan` 前后端适配增强设计

**日期**: 2026-03-04  
**状态**: 已评审（用户确认）  
**范围**: `agent-tool`, `desktop`

## 1. 背景与问题

当前 `update_plan` 工具输出结构较简化（`title/description/tasks/is_streaming`），前端虽可渲染计划，但主要是“标题 + 任务列表”模式，无法稳定承载更丰富的“要做的事情展示”（如 overview、分节、CTA）。

现状问题：
1. 前端需要额外拼装逻辑，展示语义不足。
2. 状态表达较粗，无法区分 `blocked/failed` 等执行状态。
3. 未来扩展会继续依赖非结构化文本回退，稳定性不足。

## 2. 目标与非目标

### 2.1 目标
1. 在保持向后兼容前提下扩展 `update_plan` 协议。
2. 支持前端完整展示“计划概览 + 关键步骤 + 任务状态 + 可选动作”。
3. 明确任务级与计划级状态模型，提升过程可视化。
4. 前端具备强回退能力：新字段缺失时不影响旧渲染路径。

### 2.2 非目标
1. 不在本次引入复杂富文本/Markdown 解析协议。
2. 不在本次引入独立 plan 事件流（沿用现有 `tool_call_completed` 解析路径）。
3. 不在本次做交互工作流重构（CTA 先支持基础动作）。

## 3. 方案比较

### 方案 A（采用）: 在 `plan` 内扩展可选 `view`
- 保留旧字段，新增可选 `view`、`lifecycle_status`、`progress`。
- 前端优先渲染新结构，缺失时回退旧结构。

优点：
1. 兼容性最佳，迁移风险最低。
2. 改动集中在已有数据通道，便于落地。
3. 可渐进发布，支持灰度。

代价：
1. `update_plan` 输出体增大。
2. 需要前后端同步补充校验与降级逻辑。

### 方案 B: 新增独立 `plan_view_*` 事件流
优点：语义清晰、可流式增量。  
代价：协议与 reducer 改动面过大，本次不采用。

### 方案 C: 将展示信息塞入 `description` 文本再前端解析
优点：后端改动少。  
代价：脆弱、不可预测、长期维护差，本次不采用。

## 4. 最终数据契约（兼容增强）

### 4.1 输出结构

```json
{
  "plan": {
    "title": "Execution Plan",
    "description": "Migrate plan panel rendering",
    "tasks": [
      { "id": "task-1", "title": "Define contract", "status": "completed" },
      { "id": "task-2", "title": "Implement parser", "status": "in_progress" },
      { "id": "task-3", "title": "Fix UI edge cases", "status": "blocked" }
    ],
    "is_streaming": true,
    "lifecycle_status": "in_progress",
    "progress": { "completed": 1, "total": 3, "percent": 33 },
    "view": {
      "overview": "Show plan as structured panel",
      "sections": [
        { "id": "key-steps", "title": "Key Steps", "kind": "bullets", "items": ["..."] }
      ],
      "cta": { "label": "Build", "shortcut": "⌘↩", "action": "submit" }
    }
  }
}
```

### 4.2 兼容约束
1. 旧字段 `title/description/tasks/is_streaming` 继续保留。
2. `view`、`lifecycle_status`、`progress` 全部可选。
3. 前端必须允许缺失字段并回退旧渲染。

### 4.3 状态模型

#### 任务级状态（本次确认）
- `pending`
- `in_progress`
- `blocked`
- `completed`
- `failed`

#### 计划级状态
- `planning`
- `in_progress`
- `paused`
- `completed`
- `failed`
- `cancelled`

## 5. 前端设计

### 5.1 渲染优先级
1. 若 `plan.view` 存在：渲染 `overview + sections + tasks + cta`。
2. 若 `plan.view` 缺失：维持当前 `title + progress + tasks`。

### 5.2 Header 设计
1. 保留 `PlanTitle/PlanDescription`。
2. 新增计划状态徽标（来自 `lifecycle_status`）。
3. 进度展示优先 `progress`，否则由 `tasks` 推导。

### 5.3 Task 展示
1. 任务行支持 5 态样式。
2. 每行显示状态标记 + 标题。
3. `task.description` 存在时显示次级描述。

### 5.4 Sections 展示
1. `kind=bullets` 渲染列表。
2. `kind=text` 渲染段落。
3. 未知 `kind` 忽略（容错）。

### 5.5 CTA 展示
1. `view.cta` 存在时显示 `PlanFooter + Button`。
2. `action=submit` 触发既有提交流程；`action=none` 仅展示。
3. 未知 action 降级为 no-op。

### 5.6 UI/UX 约束（基于 ui-ux-pro-max）
1. 可点击元素确保可见 hover/focus 状态。
2. 交互反馈时间 150-300ms。
3. 图标统一使用 Lucide/SVG；不使用 emoji。
4. 保持键盘可达性与最小触控尺寸。

## 6. 后端设计（`update_plan`）

### 6.1 入参扩展
在现有入参基础上新增可选字段：
1. `lifecycle_status`
2. `progress`
3. `view.overview`
4. `view.sections[]`
5. `view.cta`

### 6.2 校验规则
1. `tasks` 至少 1 条。
2. `step/title` 不能为空。
3. `task.status` 必须在允许集合内。
4. `in_progress` 最多 1 条。
5. `progress` 必须满足：
   - `total >= 0`
   - `completed >= 0`
   - `completed <= total`
   - `percent` 在 `[0, 100]`

### 6.3 默认推导
当调用方未提供时：
1. `lifecycle_status` 自动推导：
   - 有 `in_progress` => `in_progress`
   - 全部 `completed` => `completed`
   - 有 `failed` => `failed`
   - 其他 => `planning`
2. `progress` 由任务聚合推导并回填。

## 7. 数据流

1. 模型调用 `update_plan`（带新/旧字段）。
2. `agent-tool` 校验并输出兼容增强的 `output.plan`。
3. 前端 `chat-store` 在 `tool_call_completed` 中解析 `output.plan`。
4. `turn-process-sections` 渲染结构化计划；若字段缺失则回退旧模式。

## 8. 错误处理与降级

1. 后端参数错误返回 `InvalidArgs`，前端按工具失败路径展示错误信息。
2. 前端遇到未知状态/未知 section kind/action 不抛错，安全降级。
3. 任何新字段缺失都不影响旧计划展示。

## 9. 测试策略

### 9.1 后端（Rust）
1. 状态集合校验（含 `blocked/failed`）。
2. `in_progress` 多条拒绝。
3. `progress` 边界校验。
4. `view` 可选字段 roundtrip 输出。
5. 兼容老 payload（无新字段）仍通过。

### 9.2 前端（Vitest/TS）
1. 解析新协议：`view/lifecycle_status/progress`。
2. 缺失 `view` 时回退旧渲染。
3. 未知状态/未知 kind 的降级行为。
4. Plan 区域渲染快照与交互测试（展开/收起、CTA 行为）。

## 10. 发布与迁移

1. 第一阶段：后端先支持扩展字段并保持兼容。
2. 第二阶段：前端启用新渲染并保留回退逻辑。
3. 第三阶段：观察稳定性后再评估是否收敛回退分支。

## 11. 验收标准

1. 前端能展示“计划概览 + 分节 + 五态任务 + 可选 CTA”。
2. 旧 `update_plan` 输出在新前端仍正常显示。
3. 新协议在无 `view` 情况下仍可回退显示任务列表。
4. 无白屏/解析崩溃，未知字段可安全忽略。

