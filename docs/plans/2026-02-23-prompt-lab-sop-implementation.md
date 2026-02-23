# PromptLab SOP 模块实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 SOP 自动化操作流程模块，支持事件触发和手动触发，检测→处理→验证→回退的完整自动化能力。

**Architecture:** 在 prompt_lab_core 添加 SOP 相关 domain、repository、service，在 argusx-desktop 添加 SOP 前端页面，通过 Tauri IPC 通信。

**Tech Stack:** Rust (prompt_lab_core) + SQLite + Next.js + Tauri

---

## 阶段 1: 数据库迁移

### Task 1: 创建 SOP 迁移文件

**Files:**
- Create: `prompt_lab_core/migrations/202602230001_add_sops.up.sql`
- Create: `prompt_lab_core/migrations/202602230001_add_sops.down.sql`

**Step 1: 写入迁移 SQL**

```sql
-- 202602230001_add_sops.up.sql
PRAGMA foreign_keys = ON;

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
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'draft')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_sops_status ON sops(status);
CREATE INDEX idx_sops_ticket ON sops(ticket_id);

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

```sql
-- 202602230001_add_sops.down.sql
DROP TABLE IF EXISTS sop_steps;
DROP TABLE IF EXISTS sops;
```

**Step 2: 提交**

```bash
git add prompt_lab_core/migrations/202602230001_add_sops.up.sql prompt_lab_core/migrations/202602230001_add_sops.down.sql
git commit -m "feat(sop): add sops and sop_steps tables migration"
```

---

## 阶段 2: Rust Domain 定义

### Task 2: 添加 SOP Domain 类型

**Files:**
- Modify: `prompt_lab_core/src/domain.rs`

**Step 1: 添加 SopStatus 枚举和 SopStep 类型**

在 `domain.rs` 文件末尾添加：

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SopStatus {
    Active,
    Inactive,
    Draft,
}

impl SopStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Draft => "draft",
        }
    }
}

impl fmt::Display for SopStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SopStatus {
    type Err = PromptLabError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "draft" => Ok(Self::Draft),
            _ => Err(PromptLabError::InvalidEnum {
                field: "status",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sop {
    pub id: i64,
    pub sop_id: String,
    pub name: String,
    pub ticket_id: Option<String>,
    pub version: i64,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: SopStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateSopInput {
    pub sop_id: String,
    pub name: String,
    pub ticket_id: Option<String>,
    pub version: Option<i64>,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: SopStatus,
}

#[derive(Debug, Clone)]
pub struct UpdateSopInput {
    pub id: i64,
    pub sop_id: Option<String>,
    pub name: Option<String>,
    pub ticket_id: Option<String>,
    pub version: Option<i64>,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: Option<SopStatus>,
}

#[derive(Debug, Clone)]
pub struct SopFilter {
    pub status: Option<SopStatus>,
    pub ticket_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SopStep {
    pub id: i64,
    pub sop_id: String,
    pub name: String,
    pub version: i64,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateSopStepInput {
    pub sop_id: String,
    pub name: String,
    pub version: Option<i64>,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct UpdateSopStepInput {
    pub id: i64,
    pub name: Option<String>,
    pub version: Option<i64>,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct SopStepFilter {
    pub sop_id: Option<String>,
}
```

**Step 2: 提交**

```bash
git add prompt_lab_core/src/domain.rs
git commit -m "feat(sop): add Sop and SopStep domain types"
```

---

### Task 3: 添加 SOP Repository 方法

**Files:**
- Modify: `prompt_lab_core/src/repository.rs`

**Step 1: 添加 SOP Repository 方法**

在 `repository.rs` 文件末尾添加：

```rust
// ============== SOP Repository ==============

impl PromptLabRepository {
    pub async fn create_sop(&self, input: CreateSopInput) -> Result<Sop> {
        let detect = input.detect.map(|v| v.to_string());
        let handle = input.handle.map(|v| v.to_string());
        let verification = input.verification.map(|v| v.to_string());
        let rollback = input.rollback.map(|v| v.to_string());

        let row = sqlx::query_as::<_, SopRow>(
            r#"
            INSERT INTO sops (sop_id, name, ticket_id, version, detect, handle, verification, rollback, status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            RETURNING id, sop_id, name, ticket_id, version, detect, handle, verification, rollback, status, created_at, updated_at
            "#,
        )
        .bind(input.sop_id)
        .bind(input.name)
        .bind(input.ticket_id)
        .bind(input.version.unwrap_or(1))
        .bind(detect)
        .bind(handle)
        .bind(verification)
        .bind(rollback)
        .bind(input.status.as_str())
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn update_sop(&self, input: UpdateSopInput) -> Result<Sop> {
        let detect = input.detect.map(|v| v.to_string());
        let handle = input.handle.map(|v| v.to_string());
        let verification = input.verification.map(|v| v.to_string());
        let rollback = input.rollback.map(|v| v.to_string());

        let row = sqlx::query_as::<_, SopRow>(
            r#"
            UPDATE sops
            SET
              sop_id = COALESCE(?2, sop_id),
              name = COALESCE(?3, name),
              ticket_id = COALESCE(?4, ticket_id),
              version = COALESCE(?5, version),
              detect = COALESCE(?6, detect),
              handle = COALESCE(?7, handle),
              verification = COALESCE(?8, verification),
              rollback = COALESCE(?9, rollback),
              status = COALESCE(?10, status)
            WHERE id = ?1
            RETURNING id, sop_id, name, ticket_id, version, detect, handle, verification, rollback, status, created_at, updated_at
            "#,
        )
        .bind(input.id)
        .bind(input.sop_id)
        .bind(input.name)
        .bind(input.ticket_id)
        .bind(input.version)
        .bind(detect)
        .bind(handle)
        .bind(verification)
        .bind(rollback)
        .bind(input.status.map(|v| v.as_str().to_string()))
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(PromptLabError::NotFound { entity: "sops", id: input.id })?;
        row.try_into()
    }

    pub async fn list_sops(&self, filter: SopFilter) -> Result<Vec<Sop>> {
        let status = filter.status.map(|v| v.as_str().to_string());

        let rows = sqlx::query_as::<_, SopRow>(
            r#"
            SELECT id, sop_id, name, ticket_id, version, detect, handle, verification, rollback, status, created_at, updated_at
            FROM sops
            WHERE status = COALESCE(?1, status)
              AND ticket_id = COALESCE(?2, ticket_id)
            ORDER BY id DESC
            "#,
        )
        .bind(status)
        .bind(filter.ticket_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_sop(&self, id: i64) -> Result<Sop> {
        let row = sqlx::query_as::<_, SopRow>(
            r#"
            SELECT id, sop_id, name, ticket_id, version, detect, handle, verification, rollback, status, created_at, updated_at
            FROM sops
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(PromptLabError::NotFound { entity: "sops", id })?;
        row.try_into()
    }

    pub async fn get_sop_by_sop_id(&self, sop_id: &str) -> Result<Sop> {
        let row = sqlx::query_as::<_, SopRow>(
            r#"
            SELECT id, sop_id, name, ticket_id, version, detect, handle, verification, rollback, status, created_at, updated_at
            FROM sops
            WHERE sop_id = ?1
            "#,
        )
        .bind(sop_id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(PromptLabError::NotFound { entity: "sops", id: 0 })?;
        row.try_into()
    }

    pub async fn delete_sop(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM sops WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(PromptLabError::NotFound { entity: "sops", id });
        }
        Ok(())
    }

    pub async fn create_sop_step(&self, input: CreateSopStepInput) -> Result<SopStep> {
        let operation = input.operation.map(|v| v.to_string());
        let verification = input.verification.map(|v| v.to_string());
        let impact_analysis = input.impact_analysis.map(|v| v.to_string());
        let rollback = input.rollback.map(|v| v.to_string());

        let row = sqlx::query_as::<_, SopStepRow>(
            r#"
            INSERT INTO sop_steps (sop_id, name, version, operation, verification, impact_analysis, rollback)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING id, sop_id, name, version, operation, verification, impact_analysis, rollback, created_at, updated_at
            "#,
        )
        .bind(input.sop_id)
        .bind(input.name)
        .bind(input.version.unwrap_or(1))
        .bind(operation)
        .bind(verification)
        .bind(impact_analysis)
        .bind(rollback)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn update_sop_step(&self, input: UpdateSopStepInput) -> Result<SopStep> {
        let operation = input.operation.map(|v| v.to_string());
        let verification = input.verification.map(|v| v.to_string());
        let impact_analysis = input.impact_analysis.map(|v| v.to_string());
        let rollback = input.rollback.map(|v| v.to_string());

        let row = sqlx::query_as::<_, SopStepRow>(
            r#"
            UPDATE sop_steps
            SET
              name = COALESCE(?2, name),
              version = COALESCE(?3, version),
              operation = COALESCE(?4, operation),
              verification = COALESCE(?5, verification),
              impact_analysis = COALESCE(?6, impact_analysis),
              rollback = COALESCE(?7, rollback)
            WHERE id = ?1
            RETURNING id, sop_id, name, version, operation, verification, impact_analysis, rollback, created_at, updated_at
            "#,
        )
        .bind(input.id)
        .bind(input.name)
        .bind(input.version)
        .bind(operation)
        .bind(verification)
        .bind(impact_analysis)
        .bind(rollback)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(PromptLabError::NotFound { entity: "sop_steps", id: input.id })?;
        row.try_into()
    }

    pub async fn list_sop_steps(&self, filter: SopStepFilter) -> Result<Vec<SopStep>> {
        let rows = sqlx::query_as::<_, SopStepRow>(
            r#"
            SELECT id, sop_id, name, version, operation, verification, impact_analysis, rollback, created_at, updated_at
            FROM sop_steps
            WHERE sop_id = COALESCE(?1, sop_id)
            ORDER BY id ASC
            "#,
        )
        .bind(filter.sop_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_sop_step(&self, id: i64) -> Result<SopStep> {
        let row = sqlx::query_as::<_, SopStepRow>(
            r#"
            SELECT id, sop_id, name, version, operation, verification, impact_analysis, rollback, created_at, updated_at
            FROM sop_steps
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(PromptLabError::NotFound { entity: "sop_steps", id })?;
        row.try_into()
    }

    pub async fn delete_sop_step(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM sop_steps WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(PromptLabError::NotFound { entity: "sop_steps", id });
        }
        Ok(())
    }
}

// ============== Row Types ==============

#[derive(Debug, FromRow)]
struct SopRow {
    id: i64,
    sop_id: String,
    name: String,
    ticket_id: Option<String>,
    version: i64,
    detect: Option<String>,
    handle: Option<String>,
    verification: Option<String>,
    rollback: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<SopRow> for Sop {
    type Error = PromptLabError;

    fn try_from(row: SopRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            sop_id: row.sop_id,
            name: row.name,
            ticket_id: row.ticket_id,
            version: row.version,
            detect: parse_json_option(row.detect)?,
            handle: parse_json_option(row.handle)?,
            verification: parse_json_option(row.verification)?,
            rollback: parse_json_option(row.rollback)?,
            status: row.status.parse()?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, FromRow)]
struct SopStepRow {
    id: i64,
    sop_id: String,
    name: String,
    version: i64,
    operation: Option<String>,
    verification: Option<String>,
    impact_analysis: Option<String>,
    rollback: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<SopStepRow> for SopStep {
    type Error = PromptLabError;

    fn try_from(row: SopStepRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            sop_id: row.sop_id,
            name: row.name,
            version: row.version,
            operation: parse_json_option(row.operation)?,
            verification: parse_json_option(row.verification)?,
            impact_analysis: parse_json_option(row.impact_analysis)?,
            rollback: parse_json_option(row.rollback)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
```

**Step 2: 提交**

```bash
git add prompt_lab_core/src/repository.rs
git commit -m "feat(sop): add SOP repository methods"
```

---

### Task 4: 添加 SOP Service

**Files:**
- Modify: `prompt_lab_core/src/service.rs`

**Step 1: 添加 SOP Service**

在 `service.rs` 添加 SopService：

```rust
pub struct SopService {
    repo: Arc<PromptLabRepository>,
}

impl SopService {
    pub fn new(repo: Arc<PromptLabRepository>) -> Self {
        Self { repo }
    }

    pub async fn create_sop(&self, input: CreateSopInput) -> Result<Sop> {
        self.repo.create_sop(input).await
    }

    pub async fn update_sop(&self, input: UpdateSopInput) -> Result<Sop> {
        self.repo.update_sop(input).await
    }

    pub async fn list_sops(&self, filter: SopFilter) -> Result<Vec<Sop>> {
        self.repo.list_sops(filter).await
    }

    pub async fn get_sop(&self, id: i64) -> Result<Sop> {
        self.repo.get_sop(id).await
    }

    pub async fn get_sop_by_sop_id(&self, sop_id: &str) -> Result<Sop> {
        self.repo.get_sop_by_sop_id(sop_id).await
    }

    pub async fn delete_sop(&self, id: i64) -> Result<()> {
        self.repo.delete_sop(id).await
    }

    pub async fn create_sop_step(&self, input: CreateSopStepInput) -> Result<SopStep> {
        self.repo.create_sop_step(input).await
    }

    pub async fn update_sop_step(&self, input: UpdateSopStepInput) -> Result<SopStep> {
        self.repo.update_sop_step(input).await
    }

    pub async fn list_sop_steps(&self, filter: SopStepFilter) -> Result<Vec<SopStep>> {
        self.repo.list_sop_steps(filter).await
    }

    pub async fn get_sop_step(&self, id: i64) -> Result<SopStep> {
        self.repo.get_sop_step(id).await
    }

    pub async fn delete_sop_step(&self, id: i64) -> Result<()> {
        self.repo.delete_sop_step(id).await
    }
}
```

**Step 2: 更新 lib.rs 导出**

修改 `prompt_lab_core/src/lib.rs`:

```rust
pub use service::{AiLogService, CheckResultService, ChecklistService, GoldenSetService, SopService};
```

```rust
pub fn sop_service(&self) -> SopService {
    SopService::new(self.repo.clone())
}
```

**Step 3: 提交**

```bash
git add prompt_lab_core/src/service.rs prompt_lab_core/src/lib.rs
git commit -m "feat(sop): add SopService"
```

---

## 阶段 3: Tauri 命令

### Task 5: 添加 Tauri 命令

**Files:**
- Modify: `argusx-desktop/src-tauri/src/lib.rs`

**Step 1: 添加 Tauri 命令**

在 `lib.rs` 添加：

```rust
#[tauri::command]
async fn create_sop(
    state: State<'_, AppState>,
    input: CreateSopInput,
) -> Result<Sop, String> {
    state.prompt_lab.sop_service().create_sop(input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_sop(
    state: State<'_, AppState>,
    input: UpdateSopInput,
) -> Result<Sop, String> {
    state.prompt_lab.sop_service().update_sop(input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_sops(
    state: State<'_, AppState>,
    filter: SopFilter,
) -> Result<Vec<Sop>, String> {
    state.prompt_lab.sop_service().list_sops(filter).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_sop(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Sop, String> {
    state.prompt_lab.sop_service().get_sop(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_sop(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state.prompt_lab.sop_service().delete_sop(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_sop_step(
    state: State<'_, AppState>,
    input: CreateSopStepInput,
) -> Result<SopStep, String> {
    state.prompt_lab.sop_service().create_sop_step(input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_sop_step(
    state: State<'_, AppState>,
    input: UpdateSopStepInput,
) -> Result<SopStep, String> {
    state.prompt_lab.sop_service().update_sop_step(input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_sop_steps(
    state: State<'_, AppState>,
    filter: SopStepFilter,
) -> Result<Vec<SopStep>, String> {
    state.prompt_lab.sop_service().list_sop_steps(filter).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_sop_step(
    state: State<'_, AppState>,
    id: i64,
) -> Result<SopStep, String> {
    state.prompt_lab.sop_service().get_sop_step(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_sop_step(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state.prompt_lab.sop_service().delete_sop_step(id).await.map_err(|e| e.to_string())
}
```

**Step 2: 注册命令**

在 `invoke_handler` 中添加所有 SOP 命令。

**Step 3: 提交**

```bash
git add argusx-desktop/src-tauri/src/lib.rs
git commit -m "feat(sop): add Tauri commands for SOP"
```

---

## 阶段 4: 前端 API

### Task 6: 添加前端类型和 API

**Files:**
- Modify: `argusx-desktop/lib/api/prompt-lab.ts`

**Step 1: 添加 SOP 类型和 API 函数**

```typescript
// ============== SOP Types ==============

export type SopStatus = "active" | "inactive" | "draft";

export interface Sop {
  id: number;
  sop_id: string;
  name: string;
  ticket_id: string | null;
  version: number;
  detect: Record<string, unknown> | null;
  handle: Record<string, unknown> | null;
  verification: Record<string, unknown> | null;
  rollback: Record<string, unknown> | null;
  status: SopStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateSopInput {
  sop_id: string;
  name: string;
  ticket_id?: string;
  version?: number;
  detect?: Record<string, unknown>;
  handle?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
  status: SopStatus;
}

export interface UpdateSopInput {
  id: number;
  sop_id?: string;
  name?: string;
  ticket_id?: string;
  version?: number;
  detect?: Record<string, unknown>;
  handle?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
  status?: SopStatus;
}

export interface SopFilter {
  status?: SopStatus;
  ticket_id?: string;
}

export interface SopStep {
  id: number;
  sop_id: string;
  name: string;
  version: number;
  operation: Record<string, unknown> | null;
  verification: Record<string, unknown> | null;
  impact_analysis: Record<string, unknown> | null;
  rollback: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
}

export interface CreateSopStepInput {
  sop_id: string;
  name: string;
  version?: number;
  operation?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  impact_analysis?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
}

export interface UpdateSopStepInput {
  id: number;
  name?: string;
  version?: number;
  operation?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  impact_analysis?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
}

export interface SopStepFilter {
  sop_id?: string;
}

// ============== SOP API Functions ==============

export async function createSop(input: CreateSopInput): Promise<Sop> {
  return invoke("create_sop", { input });
}

export async function updateSop(input: UpdateSopInput): Promise<Sop> {
  return invoke("update_sop", { input });
}

export async function listSops(filter: SopFilter = {}): Promise<Sop[]> {
  return invoke("list_sops", { filter });
}

export async function getSop(id: number): Promise<Sop> {
  return invoke("get_sop", { id });
}

export async function deleteSop(id: number): Promise<void> {
  return invoke("delete_sop", { id });
}

export async function createSopStep(input: CreateSopStepInput): Promise<SopStep> {
  return invoke("create_sop_step", { input });
}

export async function updateSopStep(input: UpdateSopStepInput): Promise<SopStep> {
  return invoke("update_sop_step", { input });
}

export async function listSopSteps(filter: SopStepFilter = {}): Promise<SopStep[]> {
  return invoke("list_sop_steps", { filter });
}

export async function getSopStep(id: number): Promise<SopStep> {
  return invoke("get_sop_step", { id });
}

export async function deleteSopStep(id: number): Promise<void> {
  return invoke("delete_sop_step", { id });
}
```

**Step 2: 提交**

```bash
git add argusx-desktop/lib/api/prompt-lab.ts
git commit -m "feat(sop): add frontend API for SOP"
```

---

## 阶段 5: 前端页面

### Task 7: 创建 SOP 列表页

**Files:**
- Create: `argusx-desktop/app/prompt-lab/sops/page.tsx`

**Step 1: 创建页面**

参考现有模块页面结构创建 SOP 列表页。

**Step 2: 提交**

```bash
git add argusx-desktop/app/prompt-lab/sops/page.tsx
git commit -m "feat(sop): add SOP list page"
```

### Task 8: 创建 SOP 详情页

**Files:**
- Create: `argusx-desktop/app/prompt-lab/sops/[id]/page.tsx`

**Step 1: 创建页面**

**Step 2: 提交**

```bash
git add argusx-desktop/app/prompt-lab/sops/[id]/page.tsx
git commit -m "feat(sop): add SOP detail page"
```

### Task 9: 更新 Dashboard 添加 SOP 模块入口

**Files:**
- Modify: `argusx-desktop/app/prompt-lab/page.tsx`

**Step 1: 添加 SOP 模块到九宫格**

**Step 2: 提交**

```bash
git add argusx-desktop/app/prompt-lab/page.tsx
git commit -m "feat(sop): add SOP module to dashboard"
```

---

## 执行选项

**Plan complete and saved to `docs/plans/2026-02-23-prompt-lab-sop-implementation.md`. Two execution options:**

1. **Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

2. **Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
