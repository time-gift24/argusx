# PromptLab SOP 模块设计文档

**日期**: 2026-02-23
**状态**: 已批准

## 1. 概述

SOP (Standard Operating Procedure) 模块作为 PromptLab 的独立模块，支持自动化操作流程。通过事件触发或手动触发执行 SOP，实现检测→处理→验证→回退的完整自动化能力。

## 2. 触发方式

- **事件触发**: 监听工单变更事件自动执行
- **手动触发**: 用户在页面点击执行

## 3. 数据库设计

### 3.1 SOP 主表

```sql
CREATE TABLE sops (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sop_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  ticket_id TEXT,
  version INTEGER NOT NULL DEFAULT 1,
  -- JSON数组: [{"step_id": 1, "version": 1}]
  detect TEXT,    -- 操作检测步骤引用
  handle TEXT,   -- 操作处理步骤引用
  verification TEXT, -- 操作验证步骤引用
  rollback TEXT, -- 操作回退步骤引用
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'inactive', 'draft')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_sops_status ON sops(status);
CREATE INDEX idx_sops_ticket ON sops(ticket_id);
```

### 3.2 SOP 步骤表

```sql
CREATE TABLE sop_steps (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sop_id TEXT NOT NULL,
  name TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  operation TEXT,   -- 核心执行逻辑 (JSON)
  verification TEXT, -- 验证逻辑 (JSON)
  impact_analysis TEXT, -- 影响分析 (JSON)
  rollback TEXT,   -- 回退逻辑 (JSON)
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (sop_id) REFERENCES sops(sop_id)
);

CREATE INDEX idx_sop_steps_sop ON sop_steps(sop_id);
```

### 3.3 Checklist Items 与 SOP 的关联

现有 `checklist_items.target_level` 已支持 `'step'` 和 `'sop'` 值，扩展检测目标：

| 检测目标 | context_type | context_id | 说明 |
|----------|--------------|------------|------|
| SOP 流程 | `sop` | sops.id | 检测整个 SOP 流程 |
| SOP 步骤 | `sop_step` | sop_steps.id | 检测单个 SOP 步骤 |
| 步骤操作 | `sop_step_operation` | sop_steps.id | 检测步骤的 operation 字段 |
| 步骤验证 | `sop_step_verification` | sop_steps.id | 检测步骤的 verification 字段 |
| 步骤影响分析 | `sop_step_impact_analysis` | sop_steps.id | 检测步骤的 impact_analysis 字段 |
| 步骤回退 | `sop_step_rollback` | sop_steps.id | 检测步骤的 rollback 字段 |

执行结果统一写入 `check_results`，复用现有模块：
- `context_type` = 对应上表的检测目标
- `context_id` = 对应表的主键 ID
- `result` = 执行结果 (JSON)
- `is_pass` = 是否通过

执行日志复用 `ai_execution_logs`。

## 4. 目录结构

```
app/prompt-lab/
├── sops/                    # SOP 模块
│   ├── page.tsx             # SOP 列表页
│   ├── [id]/page.tsx        # SOP 详情/编辑页
│   └── execution/           # 执行记录
└── ...
```

## 5. 执行流程

1. **触发执行**: 事件触发或手动触发
2. **按序执行**: detect → handle → verification → rollback
3. **记录结果**: 每个步骤的执行结果写入 check_results
4. **记录日志**: 执行过程写入 ai_execution_logs

## 6. 待确认

- 无
