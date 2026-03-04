# `update_plan` 前后端适配增强设计

**日期**: 2026-03-04  
**状态**: 已评审（用户确认）  
**范围**: `agent-tool`, `desktop`

## 1. 背景与问题

当前 `update_plan` 已可承载基础计划（`title/description/tasks/is_streaming`），但前端主要停留在“标题 + 任务列表”展示，存在两类语义混杂：

1. **规划语义**：AI 打算做什么（目标、关键步骤、阶段策略）。
2. **执行语义**：当前具体在做哪一步，哪些被阻塞/失败。

现状中“规划结果”和“执行步骤”没有明确分层，导致：

1. 执行态不够直观，用户难以快速判断当前进展。
2. `blocked/failed` 等状态表达不足。
3. 后续扩展容易继续依赖文本推断，稳定性有限。

## 2. 本次确认结论（用户已确认）

1. **向后兼容必需**：旧 `plan` 结构必须继续可用。
2. **混合模式**：任务字段必传，展示字段可选，前端有回退。
3. **任务状态集合**：`pending | in_progress | blocked | completed | failed`。
4. **双层语义**：`Plan` 展示规划结果，`Queue` 展示执行步骤。
5. **Queue 首版范围**：仅展示 TODO，不展示消息队列。
6. **Queue 呈现方式**：单列表（每行一个状态标签），不按状态分组。

## 3. 目标与非目标

### 3.1 目标

1. 扩展 `update_plan` 协议并保持兼容。
2. 让前端同时清晰展示“规划结果（Plan）+ 执行步骤（Queue）”。
3. 落实五态任务状态并形成稳定渲染约束。
4. 保证新字段缺失时可安全回退。

### 3.2 非目标

1. 本次不引入独立 `plan_view_*` 事件流。
2. 本次不引入 Queue 分组、拖拽重排等高级交互。
3. 本次不做工具调用队列（tool queue）协议重构。

## 4. 最终数据契约（兼容增强）

### 4.1 输出结构（建议）

```json
{
  "plan": {
    "title": "Execution Plan",
    "description": "Migrate plan + queue rendering",
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
    },
    "queue": {
      "todos": [
        {
          "id": "todo-1",
          "title": "Implement parser",
          "description": "Normalize lifecycle/progress",
          "status": "in_progress"
        }
      ]
    }
  }
}
```

### 4.2 兼容约束

1. 旧字段 `title/description/tasks/is_streaming` 继续保留。
2. `view`、`lifecycle_status`、`progress`、`queue` 全部可选。
3. 若缺失 `queue.todos`，前端从 `tasks` 派生 TODO 队列。
4. 未知字段允许透传或忽略，不影响基础渲染。

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

## 5. 前端架构设计

### 5.1 ViewModel 分层

建议在 `AgentTurnVM` 明确两类队列：

1. `plan`：规划结果（结构化展示）。
2. `todoQueue`：执行步骤队列（业务 TODO，五态）。

保留现有 `turn.queue.items` 作为工具调用队列（tool queue），不与 TODO queue 混用。

### 5.2 渲染顺序

`reasoning -> plan -> queue -> tools -> terminal`

语义如下：

1. `plan`：解释“准备怎么做”。
2. `queue`：显示“正在/待做什么”（主执行视图）。
3. `tools`：底层执行细节（技术视图）。

### 5.3 Plan 展示

1. `view` 存在：展示 `overview + sections + tasks + cta`。
2. `view` 缺失：回退到当前 `title + progress + tasks`。
3. `progress` 优先使用结构化字段，否则任务聚合推导。

### 5.4 Queue（TODO）展示

1. 使用 `ai-elements/queue` 组件体系。
2. 单列表展示，不做分组。
3. 每行展示：状态指示 + 标题 + 可选描述 + 状态标签。
4. 未知状态降级到 `pending`。

### 5.5 UI/UX 约束（ui-ux-pro-max）

1. 所有可交互元素有可见 hover/focus 状态。
2. 动效时长控制在 150-300ms。
3. 图标统一使用 Lucide/SVG。
4. 保持键盘可达性与紧凑信息密度。

## 6. 后端设计（`update_plan`）

### 6.1 入参扩展

在现有 `plan` 数组与 `explanation` 基础上新增可选字段：

1. `lifecycle_status`
2. `progress`
3. `view.overview`
4. `view.sections[]`
5. `view.cta`
6. `queue.todos[]`

### 6.2 校验规则

1. `tasks` 至少 1 条。
2. `step/title` 非空。
3. `task.status` 必须在五态集合中。
4. `in_progress` 最多 1 条。
5. `progress` 约束：
   - `total >= 0`
   - `completed >= 0`
   - `completed <= total`
   - `percent` 在 `[0, 100]`
6. `queue.todos[].status` 同样使用五态集合。

### 6.3 默认推导

当调用方未提供时：

1. `lifecycle_status` 自动推导：
   - 有 `in_progress` => `in_progress`
   - 全部 `completed` => `completed`
   - 有 `failed` => `failed`
   - 其他 => `planning`
2. `progress` 按任务聚合计算。
3. `queue.todos` 缺失时可由 `tasks` 映射构造（可选后端推导，前端必须具备兜底推导）。

## 7. 事件与更新规则

1. `tool_call_completed(update_plan)`：刷新 `plan` 主体并同步 `todoQueue` 快照。
2. `task_*` 事件：优先更新 `todoQueue` 的对应 `todo.id` 状态。
3. 冲突处理：按最新事件时序更新（前端以 `seq/ts` 为准）。
4. 工具队列与 TODO 队列完全解耦：
   - `turn.queue.items` = tool queue
   - `turn.todoQueue.todos` = business todo queue

## 8. 错误处理与降级

1. 后端参数错误返回 `InvalidArgs`，前端按 tool error 展示。
2. 前端遇到未知 status/kind/action 均安全降级，不抛运行时错误。
3. 新字段缺失时，`Plan` 与 `Queue` 至少能展示任务级基础信息。

## 9. 测试策略

### 9.1 后端（Rust）

1. 五态状态校验（含 `blocked/failed`）。
2. `in_progress` 多条拒绝。
3. `progress` 边界校验。
4. `view` 与 `queue.todos` 的可选字段输出。
5. 兼容旧 payload（无新字段）通过。

### 9.2 前端（Vitest/TS）

1. Store 解析：`view/lifecycle/progress/queue.todos`。
2. 缺失 `queue.todos` 时从 `tasks` 派生。
3. `task_*` 事件更新 TODO queue 行状态。
4. `turn-process-sections` 渲染 Plan + Queue 双层结构。
5. Queue 单列表五态渲染与未知状态降级。

## 10. 发布与迁移

1. 第一阶段：后端支持扩展字段并保持兼容。
2. 第二阶段：前端启用 `Plan + Queue` 双层渲染与回退逻辑。
3. 第三阶段：观测稳定性后再评估是否精简旧分支。

## 11. 验收标准

1. 前端可同时展示规划结果（Plan）与执行步骤（Queue）。
2. Queue 为 TODO 单列表，支持五态显示。
3. `turn.queue.items`（工具队列）与 TODO queue 均正常工作且互不混淆。
4. 旧 `update_plan` 输出在新前端仍可正常显示。
5. 无白屏/崩溃，未知字段可安全忽略。

