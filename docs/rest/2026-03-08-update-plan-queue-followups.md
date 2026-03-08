# update_plan Queue 遗留问题

## 遗留问题

### 模型策略层不保证强制收敛到完成态

当前 V1 只保证以下能力：

- `update_plan` 对 desktop chat runtime 可见且可调用
- 每次有效调用都会被校验
- 每次有效调用都会被转换成结构化计划快照
- 最新计划状态会被流式发送到前端并显示为 `Queue`

V1 不保证模型策略层一定会在存在剩余任务时持续调用 `update_plan`，直到所有任务都变为 `completed`。

这是一个后续的 agent loop / orchestration 问题，可能需要：

- 更强的 system prompt 约束
- step 完成后的自检与剩余任务回看
- 在 turn driver 中引入额外的策略闭环
- 对“未完成计划结束 turn”的专门收尾逻辑

本问题故意不放入当前 UI + event integration 范围，避免把 V1 范围扩展成策略层重构。
