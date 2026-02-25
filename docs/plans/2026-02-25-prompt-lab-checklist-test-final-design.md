# PromptLab Checklist 测试模块 - 最终设计文档

## 一、概述

本文档描述 PromptLab 中 checklist-item 测试功能的最终目标设计，包括数据库结构、API 接口、前端页面和业务流程。

---

## 二、数据库设计

### 2.1 核心概念

| 概念 | 说明 |
|------|------|
| **context_type** | 检查项适用的上下文类型，在创建检查项时固定，不可修改 |
| **context_key** | 具体的目标标识（哪个 SOP 或哪个 Step），TEXT 类型 |

### 2.2 Context Type 枚举

| context_type | 含义 | 需要选择的 context_key |
|-------------|------|----------------------|
| `sop` | 验证整个 SOP | SOP 的 id |
| `sop_procedure_detect` | 验证 SOP 的 detect 字段 | SOP 的 id |
| `sop_procedure_handle` | 验证 SOP 的 handle 字段 | SOP 的 id |
| `sop_procedure_verification` | 验证 SOP 的 verification 字段 | SOP 的 id |
| `sop_procedure_rollback` | 验证 SOP 的 rollback 字段 | SOP 的 id |
| `sop_step_operation` | 验证 Step 的 operation 字段 | Step 的 id |
| `sop_step_verification` | 验证 Step 的 verification 字段 | Step 的 id |
| `sop_step_impact_analysis` | 验证 Step 的 impact_analysis 字段 | Step 的 id |
| `sop_step_rollback` | 验证 Step 的 rollback 字段 | Step 的 id |
| `sop_step_common` | 通用校验，不限定具体 Step | 空 / "common" |

### 2.3 Context Key 格式

- **类型**：TEXT（字符串）
- **格式**：
  - `sop` / `sop_procedure_*`：存数字 id（如 `"123"`）
  - `sop_step_*`（非 common）：存数字 id（如 `"456"`）
  - `sop_step_common`：存 `"common"` 或空

### 2.4 完整 Schema

> 完整数据库 Schema 请参见本文档**附录**。

---

## 三、API 设计

### 3.1 检查项管理

#### 创建检查项
```
POST /checklist-items
Body: {
  name: string,
  prompt: string,
  context_type: string,        // 从 target_level 改名为 context_type
  result_schema?: object,
  status: "active" | "inactive" | "draft"
}
Response: ChecklistItem
```

#### 更新检查项
```
PATCH /checklist-items/{id}
Body: {
  name?: string,
  prompt?: string,
  context_type?: string,
  result_schema?: object,
  status?: string
}
Response: ChecklistItem
```

#### 获取检查项列表
```
GET /checklist-items?status=&context_type=
Response: ChecklistItem[]
```

### 3.2 测试执行

#### 执行检查（保存结果）
```
POST /check-results
Body: {
  context_type: string,        // 如 "sop_procedure_detect"
  context_key: string,         // 如 "123"（SOP id）
  check_item_id: number,
  source_type: "ai" | "manual",
  operator_id?: string,
  result?: object,             // JSON 结果
  is_pass: boolean
}
Response: CheckResult
```

#### 查询检查结果
```
GET /check-results?context_type=&context_key=&check_item_id=
Response: CheckResult[]
```

### 3.3 辅助接口

#### 获取 SOP 列表
```
GET /sops
Response: Sop[]
```

#### 获取 SOP Steps
```
GET /sop-steps?sop_id=
Response: SopStep[]
```

---

## 四、前端页面设计

### 4.1 页面路由
```
/prompt-lab/test
```

### 4.2 页面布局

```
┌────────────────────────────────────────────────────────────────────────┐
│  🧪 Checklist 测试                                                     │
│  选择检查项和具体内容，执行测试验证                                      │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─ 1️⃣ 选择检查项 ───────────────────────────────────────────────┐   │
│  │                                                                  │   │
│  │  [检查项下拉框 ▼]                                               │   │
│  │  ├─ 检查项 A [sop]                                             │   │
│  │  ├─ 检查项 B [sop_procedure_detect]                            │   │
│  │  └─ 检查项 C [sop_step_operation]                             │   │
│  │                                                                  │   │
│  └────────────────────────────────────────────────────────────────┘   │
│                                                                        │
│  ┌─ 2️⃣ 选择具体内容 (根据检查项的 context_type 动态显示) ──────────┐   │
│  │                                                                  │   │
│  │  情况 A: context_type = "sop*"                                  │   │
│  │  ┌─ 选择 SOP ─────────────────────────────────────────────┐   │   │
│  │  │ [SOP 下拉框 ▼]                                         │   │   │
│  │  │  ├─ SOP-001: 用户登录流程                              │   │   │
│  │  │  └─ SOP-002: 订单处理流程                              │   │   │
│  │  └─────────────────────────────────────────────────────────┘   │   │
│  │                                                                  │   │
│  │  情况 B: context_type = "sop_step_*" (非 common)               │   │
│  │  ┌─ 选择 SOP ──┐  ┌─ 选择 Step ──────────────────────────┐    │   │
│  │  │ [SOP ▼]    │  │ [Step ▼]                           │    │   │
│  │  └────────────┘  └──────────────────────────────────────┘    │   │
│  │                                                                  │   │
│  │  情况 C: context_type = "sop_step_common"                     │   │
│  │  └─ 无需选择具体内容（通用校验）                                │   │
│  │                                                                  │   │
│  └────────────────────────────────────────────────────────────────┘   │
│                                                                        │
│  [ ▶ 运行测试 ] (当所有必选项已选择时启用)                             │
│                                                                        │
│  ┌─ 📄 文档内容预览 (可折叠) ──────────────────────────────────────┐   │
│  │                                                                  │   │
│  │  {                                                              │   │
│  │    "sop_id": "SOP-001",                                        │   │
│  │    "name": "用户登录流程",                                      │   │
│  │    "detect": [...],     ← 根据 context_type 显示对应字段       │   │
│  │    "handle": [...]                                              │   │
│  │  }                                                              │   │
│  │                                                                  │   │
│  └────────────────────────────────────────────────────────────────┘   │
│                                                                        │
│  ┌─ ✅ 测试结果 (可折叠) ──────────────────────────────────────────┐   │
│  │                                                                  │   │
│  │  [✓] 测试通过    [✗] 测试失败                                  │   │
│  │                                                                  │   │
│  │  检查项: 检查登录流程是否包含权限验证                            │   │
│  │  上下文: sop_procedure_detect / SOP-001                         │   │
│  │  来源: manual                                                   │   │
│  │                                                                  │   │
│  │  {                                                              │   │
│  │    "has_permission_check": true,                               │   │
│  │    "confidence": 0.95                                          │   │
│  │  }                                                              │   │
│  │                                                                  │   │
│  └────────────────────────────────────────────────────────────────┘   │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 4.3 组件说明

| 组件 | 说明 | 交互 |
|------|------|------|
| 检查项下拉框 | 显示所有 active 状态的检查项 | 选择后自动带出 context_type |
| SOP 下拉框 | 当 context_type 以 sop 开头时显示 | 过滤 Step 下拉框的选项 |
| Step 下拉框 | 当 context_type 以 sop_step_ 开头时显示（除 common） | 只显示选中 SOP 下的 Steps |
| 运行测试按钮 | 执行检查 | 调用 API，保存结果，显示返回数据 |
| 文档预览 | 显示 JSON 格式的文档内容 | 根据选择动态更新 |
| 测试结果 | 显示通过/失败状态和详细信息 | 每次测试后更新 |

---

## 五、业务流程

### 5.1 检查项创建流程

```
1. 用户进入检查项管理页面 (/prompt-lab/checklist)
2. 点击"新建检查项"
3. 填写：
   - 名称：检查登录流程是否包含权限验证
   - 提示词：检查以下流程是否包含权限验证步骤...
   - 上下文类型：sop_procedure_detect  ← 创建时选择，不能修改
   - 结果 Schema：{"type": "object", "properties": {...}}
4. 保存
5. 检查项列表中显示该项，带有 context_type 标签
```

### 5.2 测试执行流程

```
1. 用户进入测试页面 (/prompt-lab/test)
2. 选择检查项（系统自动识别其 context_type）
3. 根据 context_type 显示对应选择：
   - sop* → 显示"SOP 选择"
   - sop_step_* → 显示"SOP 选择" + "Step 选择"
   - sop_step_common → 无需选择
4. 文档预览区域自动加载并显示对应内容
5. 点击"运行测试"
6. 系统：
   a. 构造完整的提示词（检查项 prompt + 文档内容）
   b. 调用 LLM 执行检查（如果 source_type = "ai"）
   c. 解析结果，对比 result_schema 验证格式
   d. 保存结果到 check_results 表
   e. 保存执行日志到 ai_execution_logs 表
7. 页面显示测试结果
```

### 5.3 数据流向

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Checklist │────▶│   Test      │────▶│   LLM       │
│  Item      │     │   Execution │     │   (AI)      │
│  (prompt)  │     │             │     │             │
└─────────────┘     └──────┬──────┘     └──────┬──────┘
                          │                    │
                          ▼                    ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ SOP / Step │────▶│  Combine    │────▶│   Parse     │
│ (document) │     │  Prompt     │     │   Result    │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                    ┌─────────────┐             │
                    │  Save to   │◀────────────┘
                    │  DB        │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  check_results  │ │ai_execution_logs│ │  Frontend       │
│  (is_pass,      │ │  (full log)     │ │  (display)      │
│   result)       │ │                 │ │                 │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

---

## 六、核心概念

| 概念 | 说明 |
|------|------|
| **Checklist Item** | 一个检查规则，包含 prompt 和期望的 result_schema |
| **Context Type** | 检查项适用的范围，创建时确定，不可修改 |
| **Context Key** | 具体的目标标识（哪个 SOP 或哪个 Step） |
| **Check Result** | 一次检查的结果（通过/失败 + 详细结果） |
| **Execution Log** | 完整的执行日志（输入、输出、错误） |

---

## 七、页面入口

在 PromptLab 首页 (`/prompt-lab`) 添加导航项：

| 模块 | 图标 | 路由 | 说明 |
|------|------|------|------|
| 检查项 | ✅ | /prompt-lab/checklist | 管理检查项 |
| 测试 | 🧪 | /prompt-lab/test | 执行测试 |
| Golden Sets | 📁 | /prompt-lab/golden-sets | golden set 管理 |
| 结果 | 📊 | /prompt-lab/results | 查看历史结果 |
| 日志 | 📝 | /prompt-lab/logs | 查看执行日志 |
| SOPs | 📋 | /prompt-lab/sops | SOP 管理 |

---

## 附录：完整数据库 Schema

### checklist_items（检查项表）

```sql
CREATE TABLE checklist_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  prompt TEXT NOT NULL,
  context_type TEXT NOT NULL
    CHECK (context_type IN (
      'sop',
      'sop_procedure_detect',
      'sop_procedure_handle',
      'sop_procedure_verification',
      'sop_procedure_rollback',
      'sop_step_operation',
      'sop_step_verification',
      'sop_step_impact_analysis',
      'sop_step_rollback',
      'sop_step_common'
    )),
  result_schema TEXT CHECK (result_schema IS NULL OR json_valid(result_schema)),
  version INTEGER NOT NULL DEFAULT 1,
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'inactive', 'draft')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  created_by INTEGER,
  updated_by INTEGER,
  deleted_at TEXT
);

CREATE INDEX idx_checklist_status ON checklist_items (status);
CREATE INDEX idx_checklist_context_type ON checklist_items (context_type);

CREATE TRIGGER trg_checklist_items_updated_at
AFTER UPDATE ON checklist_items
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
  UPDATE checklist_items
  SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
  WHERE id = NEW.id;
END;
```

### check_results（检查结果表）

```sql
CREATE TABLE check_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  context_type TEXT NOT NULL,
  context_key TEXT NOT NULL,
  check_item_id INTEGER NOT NULL,
  source_type INTEGER NOT NULL DEFAULT 1 CHECK (source_type IN (1, 2)),
  operator_id TEXT,
  result TEXT CHECK (result IS NULL OR json_valid(result)),
  is_pass INTEGER NOT NULL DEFAULT 0 CHECK (is_pass IN (0, 1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (check_item_id) REFERENCES checklist_items(id)
);

CREATE INDEX idx_check_results_item ON check_results (check_item_id);
CREATE INDEX idx_check_results_context ON check_results (context_type, context_key);
```

### ai_execution_logs（AI 执行日志表）

```sql
CREATE TABLE ai_execution_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  check_result_id INTEGER,
  context_type TEXT NOT NULL,
  context_key TEXT NOT NULL,
  check_item_id INTEGER NOT NULL,
  model_provider TEXT,
  model_version TEXT NOT NULL,
  temperature REAL DEFAULT 0.0,
  prompt_snapshot TEXT,
  raw_output TEXT,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  exec_status INTEGER NOT NULL DEFAULT 0 CHECK (exec_status IN (0, 1, 2, 3)),
  error_message TEXT,
  latency_ms INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (check_result_id) REFERENCES check_results(id),
  FOREIGN KEY (check_item_id) REFERENCES checklist_items(id)
);

CREATE INDEX idx_ai_logs_item ON ai_execution_logs (check_item_id);
CREATE INDEX idx_ai_logs_context ON ai_execution_logs (context_type, context_key);
```

### sops（SOP 主表）

```sql
CREATE TABLE sops (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sop_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  ticket_id TEXT,
  version INTEGER NOT NULL DEFAULT 1,
  detect TEXT,
  handle TEXT,
  verification TEXT,
  rollback TEXT,
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'inactive', 'draft')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_sops_status ON sops(status);
CREATE INDEX idx_sops_ticket ON sops(ticket_id);
```

### sop_steps（SOP 步骤表）

```sql
CREATE TABLE sop_steps (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sop_id TEXT NOT NULL,
  name TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  operation TEXT,
  verification TEXT,
  impact_analysis TEXT,
  rollback TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (sop_id) REFERENCES sops(sop_id)
);

CREATE INDEX idx_sop_steps_sop ON sop_steps(sop_id);
```

### golden_set_items（Golden Set 关联表）

```sql
CREATE TABLE golden_set_items (
  golden_set_id INTEGER NOT NULL,
  checklist_item_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  PRIMARY KEY (golden_set_id, checklist_item_id),
  FOREIGN KEY (golden_set_id) REFERENCES check_results(id),
  FOREIGN KEY (checklist_item_id) REFERENCES checklist_items(id)
);

CREATE INDEX idx_gsi_item ON golden_set_items (checklist_item_id);
```
