# Checklist 测试页面设计方案

## 一、需求概述

### 1.1 目标
新增一个 checklist-item 测试页面，允许用户选择检查项并执行测试验证。

### 1.2 核心概念

| 术语 | 说明 |
|------|------|
| `context_type` | 检查项适用的上下文类型，在创建检查项时固定 |
| `context_key` | 具体的目标标识（SOP id 或 SOP Step id），TEXT 类型 |

### 1.3 Context Type 分类

| context_type | 含义 | 需要选择的具体内容 |
|-------------|------|------------------|
| `sop` | 验证整个 SOP | 选择 SOP |
| `sop_procedure_detect` | 验证 SOP 的 detect 字段 | 选择 SOP |
| `sop_procedure_handle` | 验证 SOP 的 handle 字段 | 选择 SOP |
| `sop_procedure_verification` | 验证 SOP 的 verification 字段 | 选择 SOP |
| `sop_procedure_rollback` | 验证 SOP 的 rollback 字段 | 选择 SOP |
| `sop_step_operation` | 验证 SOP Step 的 operation 字段 | 选择 SOP + Step |
| `sop_step_verification` | 验证 SOP Step 的 verification 字段 | 选择 SOP + Step |
| `sop_step_impact_analysis` | 验证 SOP Step 的 impact_analysis 字段 | 选择 SOP + Step |
| `sop_step_rollback` | 验证 SOP Step 的 rollback 字段 | 选择 SOP + Step |
| `sop_step_common` | 通用校验，不限定具体 Step | 无需选择 |

---

## 二、前端设计方案

### 2.1 页面布局（ASCII）

```
┌─────────────────────────────────────────────────────────────────┐
│  🧪 Checklist 测试                                               │
│  选择检查项和具体内容，执行测试验证                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 📋 测试配置                                              │   │
│  │                                                          │   │
│  │ 1️⃣ 选择检查项                      [检查项下拉框      ▼]   │   │
│  │    └─ 已选: {名称} [{context_type 标签}]                │   │
│  │                                                          │   │
│  │ 2️⃣ 选择具体内容                                           │   │
│  │    ├─ 若 context_type = sop_*        [SOP 下拉框    ▼]   │   │
│  │    └─ 若 context_type = sop_step_*   [SOP 下拉框▼]     │   │
│  │                                     [Step 下拉框    ▼]   │   │
│  │                                                          │   │
│  │    [▶ 运行测试]                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─ 📄 文档内容预览 (可折叠)                                   │
│  │  ┌─────────────────────────────────────────────────────┐   │
│  │  │ {JSON 格式的文档内容}                               │   │
│  │  │ {根据选择的 SOP/Step 和 context_type 动态显示}      │   │
│  │  └─────────────────────────────────────────────────────┘   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ├─ ✅ 测试结果 (可折叠)                                        │
│  │  ┌─────────────────────────────────────────────────────┐   │
│  │  │ [✓/✗] 测试通过/失败                                │   │
│  │  │ 检查项: {名称}                                     │   │
│  │  │ 上下文: {context_type} / {SOP名称/Step名称}       │   │
│  │  │ 结果: {JSON}                                       │   │
│  │  └─────────────────────────────────────────────────────┘   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 交互流程

```
选择检查项
    │
    ▼
┌─────────────────────┐
│ 获取 context_type   │
│ (创建时已固定)      │
└─────────────────────┘
    │
    ▼
根据 context_type 显示对应选择：
├─ sop / sop_procedure_* → 显示 "选择 SOP"
├─ sop_step_* (非 common) → 显示 "选择 SOP" + "选择 Step"
└─ sop_step_common → 无需选择
    │
    ▼
加载文档内容预览
    │
    ▼
运行测试
    │
    ▼
显示测试结果
```

### 2.3 组件说明

| 区域 | 组件 | 说明 |
|------|------|------|
| 检查项选择 | Select | 下拉选择已创建的检查项，显示名称 + context_type 标签 |
| SOP 选择 | Select | 当 context_type 以 sop 开头时显示 |
| Step 选择 | Select | 当 context_type 以 sop_step_ 开头时显示（除 common） |
| 文档预览 | Collapsible + Pre | 可折叠区域，显示 JSON 格式的文档内容 |
| 测试按钮 | Button | 禁用条件：未选择检查项，或未完成必选选择 |
| 测试结果 | Collapsible + Card | 可折叠区域，显示通过/失败状态和详细结果 |

### 2.4 路由

```
/app/prompt-lab/test/page.tsx
```

需要在 `/app/prompt-lab/page.tsx` 的导航中添加链接。

---

## 三、后端改动方案

### 3.1 数据库 Migration

#### checklist_items 表
```sql
-- 重命名字段
ALTER TABLE checklist_items RENAME COLUMN target_level TO context_type;
```

#### check_results 表
```sql
-- 删除旧的 context_id，添加新的 context_key
ALTER TABLE check_results DROP COLUMN context_id;
ALTER TABLE check_results ADD COLUMN context_key TEXT NOT NULL;
```

#### ai_execution_logs 表
```sql
ALTER TABLE ai_execution_logs DROP COLUMN context_id;
ALTER TABLE ai_execution_logs ADD COLUMN context_key TEXT NOT NULL;
```

#### Index 调整
```sql
-- 重建索引
DROP INDEX IF EXISTS idx_context_ref;
CREATE INDEX idx_context_ref ON check_results (context_type, context_key);

DROP INDEX IF EXISTS idx_context_log;
CREATE INDEX idx_context_log ON ai_execution_logs (context_type, context_key);
```

### 3.2 Rust Domain 改动

**domain.rs** - 修改结构体：

```rust
// CheckResult
pub struct CheckResult {
    pub id: i64,
    pub context_type: String,      // 保持不变
    pub context_key: String,       // 原 context_id: i64
    pub check_item_id: i64,
    pub source_type: SourceType,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: bool,
    pub created_at: String,
}

// UpsertCheckResultInput
pub struct UpsertCheckResultInput {
    pub id: Option<i64>,
    pub context_type: String,
    pub context_key: String,       // 原 context_id: i64
    pub check_item_id: i64,
    pub source_type: SourceType,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: bool,
}

// CheckResultFilter
pub struct CheckResultFilter {
    pub context_type: Option<String>,
    pub context_key: Option<String>,  // 原 context_id: Option<i64>
    pub check_item_id: Option<i64>,
}
```

**UpsertChecklistItemInput / ChecklistItem**：
```rust
// 字段名从 target_level 改为 context_type
pub context_type: String,  // 原 target_level: TargetLevel
```

### 3.3 Rust Repository 改动

**repository.rs** - SQL 语句修改：

```sql
-- 原来的 context_id 改为 context_key，类型从 INTEGER 改为 TEXT
INSERT INTO check_results ...
  context_type = ?2,
  context_key = ?3,  -- 原 context_id = ?3 (INTEGER)
  ...

-- Filter 查询
WHERE context_type = COALESCE(?1, context_type)
  AND context_key = COALESCE(?2, context_key)  -- 原 AND context_id = COALESCE(?2, context_id)
```

### 3.4 API 改动

**Tauri Commands** (`src-tauri/src/lib.rs`)：
- 更新 `UpsertCheckResultInput` 结构体
- 更新 `CheckResultFilter` 结构体

**TypeScript API** (`lib/api/prompt-lab.ts`)：
- 更新 `CheckResult` 接口：`context_id` → `context_key` (string)
- 更新 `UpsertCheckResultInput` 接口
- 更新 `CheckResultFilter` 接口

### 3.5 Context Type 校验

建议在 Rust 侧添加校验，确保 context_type 是有效值：

```rust
const VALID_CONTEXT_TYPES: &[&str] = &[
    "sop",
    "sop_procedure_detect",
    "sop_procedure_handle",
    "sop_procedure_verification",
    "sop_procedure_rollback",
    "sop_step_operation",
    "sop_step_verification",
    "sop_step_impact_analysis",
    "sop_step_rollback",
    "sop_step_common",
];

fn validate_context_type(ct: &str) -> Result<()> {
    if VALID_CONTEXT_TYPES.contains(&ct) {
        Ok(())
    } else {
        Err(PromptLabError::InvalidEnum {
            field: "context_type",
            value: ct.to_string(),
        })
    }
}
```

---

## 四、文件修改清单

### 4.1 后端 (prompt_lab_core)

| 文件 | 改动 |
|------|------|
| `src/domain.rs` | `context_id` → `context_key` (String), `target_level` → `context_type` |
| `src/repository.rs` | SQL 语句中的字段名和类型更新 |
| `src/service.rs` | 可选：添加 context_type 校验 |

### 4.2 前端 (argusx-desktop)

| 文件 | 改动 |
|------|------|
| `lib/api/prompt-lab.ts` | 类型定义更新 |
| `src-tauri/src/lib.rs` | Tauri command 参数更新 |
| `app/prompt-lab/test/page.tsx` | **新增** 测试页面 |
| `app/prompt-lab/page.tsx` | 添加测试页面导航链接 |

### 4.3 文档

| 文件 | 改动 |
|------|------|
| `docs/database/prompt-lab-database.md` | 更新字段定义 |
| `docs/openapi/prompt-lab-api.yaml` | 更新 API 定义 |

---

## 五、验证方案

### 5.1 后端测试
```bash
cd prompt_lab_core
cargo test
```

### 5.2 前端验证
```bash
cd argusx-desktop
pnpm dev
# 访问 http://localhost:3000/prompt-lab/test
```

### 5.3 测试流程
1. 创建一个 context_type = `sop_procedure_detect` 的检查项
2. 进入测试页面，选择该检查项
3. 验证：自动显示"SOP 选择"（不显示 Step 选择）
4. 选择一个 SOP
5. 验证：文档预览区域显示对应的 detect JSON
6. 点击运行测试
7. 验证：显示测试结果

---

## 六、待确认问题

1. **测试执行逻辑**：当前设计只是调用 `upsertCheckResult` 保存结果，实际的 AI 检查逻辑如何触发？
2. **Golden Set 关联**：测试时是否需要加载 golden set 数据？
3. **历史记录**：是否需要在结果页面显示历史测试记录？
