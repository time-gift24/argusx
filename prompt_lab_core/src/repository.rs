use serde_json::Value;
use sqlx::{FromRow, SqlitePool};

use crate::domain::*;
use crate::error::{PromptLabError, Result};

#[derive(Debug, Clone)]
pub struct PromptLabRepository {
    pool: SqlitePool,
}

impl PromptLabRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create_checklist_item(
        &self,
        input: CreateChecklistItemInput,
    ) -> Result<ChecklistItem> {
        let result_schema = input.result_schema.map(|v| v.to_string());
        let row = sqlx::query_as::<_, ChecklistItemRow>(
            r#"
            INSERT INTO checklist_items (
              name, prompt, target_level, result_schema, version, status, created_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING
              id, name, prompt, target_level, result_schema, version, status,
              created_at, updated_at, created_by, updated_by, deleted_at
            "#,
        )
        .bind(input.name)
        .bind(input.prompt)
        .bind(input.target_level.as_str())
        .bind(result_schema)
        .bind(input.version.unwrap_or(1))
        .bind(input.status.as_str())
        .bind(input.created_by)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn update_checklist_item(
        &self,
        input: UpdateChecklistItemInput,
    ) -> Result<ChecklistItem> {
        let result_schema = input.result_schema.map(|v| v.to_string());
        let row = sqlx::query_as::<_, ChecklistItemRow>(
            r#"
            UPDATE checklist_items
            SET
              name = COALESCE(?2, name),
              prompt = COALESCE(?3, prompt),
              target_level = COALESCE(?4, target_level),
              result_schema = COALESCE(?5, result_schema),
              version = COALESCE(?6, version),
              status = COALESCE(?7, status),
              updated_by = COALESCE(?8, updated_by)
            WHERE id = ?1 AND deleted_at IS NULL
            RETURNING
              id, name, prompt, target_level, result_schema, version, status,
              created_at, updated_at, created_by, updated_by, deleted_at
            "#,
        )
        .bind(input.id)
        .bind(input.name)
        .bind(input.prompt)
        .bind(input.target_level.map(|v| v.as_str().to_string()))
        .bind(result_schema)
        .bind(input.version)
        .bind(input.status.map(|v| v.as_str().to_string()))
        .bind(input.updated_by)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(PromptLabError::NotFound {
            entity: "checklist_items",
            id: input.id,
        })?;

        row.try_into()
    }

    pub async fn list_checklist_items(
        &self,
        filter: ChecklistFilter,
    ) -> Result<Vec<ChecklistItem>> {
        let status = filter.status.map(|v| v.as_str().to_string());
        let target_level = filter.target_level.map(|v| v.as_str().to_string());

        let rows = sqlx::query_as::<_, ChecklistItemRow>(
            r#"
            SELECT
              id, name, prompt, target_level, result_schema, version, status,
              created_at, updated_at, created_by, updated_by, deleted_at
            FROM checklist_items
            WHERE deleted_at IS NULL
              AND status = COALESCE(?1, status)
              AND target_level = COALESCE(?2, target_level)
            ORDER BY id DESC
            "#,
        )
        .bind(status)
        .bind(target_level)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_checklist_item(&self, id: i64) -> Result<ChecklistItem> {
        let row = sqlx::query_as::<_, ChecklistItemRow>(
            r#"
        SELECT
          id, name, prompt, target_level, result_schema, version, status,
          created_at, updated_at, created_by, updated_by, deleted_at
        FROM checklist_items
        WHERE id = ?1 AND deleted_at IS NULL
        "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(PromptLabError::NotFound {
            entity: "checklist_items",
            id,
        })?;

        row.try_into()
    }

    pub async fn bind_golden_set_item(
        &self,
        input: BindGoldenSetItemInput,
    ) -> Result<GoldenSetItem> {
        let row = sqlx::query_as::<_, GoldenSetItemRow>(
            r#"
            INSERT INTO golden_set_items (golden_set_id, checklist_item_id, sort_order)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(golden_set_id, checklist_item_id)
            DO UPDATE SET sort_order = excluded.sort_order
            RETURNING golden_set_id, checklist_item_id, sort_order, created_at
            "#,
        )
        .bind(input.golden_set_id)
        .bind(input.checklist_item_id)
        .bind(input.sort_order)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn list_golden_set_items(&self, golden_set_id: i64) -> Result<Vec<GoldenSetItem>> {
        let rows = sqlx::query_as::<_, GoldenSetItemRow>(
            r#"
            SELECT golden_set_id, checklist_item_id, sort_order, created_at
            FROM golden_set_items
            WHERE golden_set_id = ?1
            ORDER BY sort_order ASC, checklist_item_id ASC
            "#,
        )
        .bind(golden_set_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn unbind_golden_set_item(
        &self,
        golden_set_id: i64,
        checklist_item_id: i64,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM golden_set_items
            WHERE golden_set_id = ?1 AND checklist_item_id = ?2
            "#,
        )
        .bind(golden_set_id)
        .bind(checklist_item_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(PromptLabError::NotFound {
                entity: "golden_set_items",
                id: golden_set_id,
            });
        }
        Ok(())
    }

    pub async fn soft_delete_checklist_item(&self, id: i64) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE checklist_items
            SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(PromptLabError::NotFound {
                entity: "checklist_items",
                id,
            });
        }
        Ok(())
    }

    pub async fn upsert_check_result(&self, input: UpsertCheckResultInput) -> Result<CheckResult> {
        let result = input.result.map(|v| v.to_string());

        let row = if let Some(id) = input.id {
            sqlx::query_as::<_, CheckResultRow>(
                r#"
                UPDATE check_results
                SET
                  context_type = ?2,
                  context_id = ?3,
                  check_item_id = ?4,
                  source_type = ?5,
                  operator_id = ?6,
                  result = ?7,
                  is_pass = ?8
                WHERE id = ?1
                RETURNING id, context_type, context_id, check_item_id, source_type,
                          operator_id, result, is_pass, created_at
                "#,
            )
            .bind(id)
            .bind(input.context_type)
            .bind(input.context_id)
            .bind(input.check_item_id)
            .bind(input.source_type.as_i64())
            .bind(input.operator_id)
            .bind(result)
            .bind(if input.is_pass { 1_i64 } else { 0_i64 })
            .fetch_optional(&self.pool)
            .await?
            .ok_or(PromptLabError::NotFound {
                entity: "check_results",
                id,
            })?
        } else {
            sqlx::query_as::<_, CheckResultRow>(
                r#"
                INSERT INTO check_results (
                  context_type, context_id, check_item_id, source_type,
                  operator_id, result, is_pass
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                RETURNING id, context_type, context_id, check_item_id, source_type,
                          operator_id, result, is_pass, created_at
                "#,
            )
            .bind(input.context_type)
            .bind(input.context_id)
            .bind(input.check_item_id)
            .bind(input.source_type.as_i64())
            .bind(input.operator_id)
            .bind(result)
            .bind(if input.is_pass { 1_i64 } else { 0_i64 })
            .fetch_one(&self.pool)
            .await?
        };

        row.try_into()
    }

    pub async fn list_check_results(&self, filter: CheckResultFilter) -> Result<Vec<CheckResult>> {
        let rows = sqlx::query_as::<_, CheckResultRow>(
            r#"
            SELECT id, context_type, context_id, check_item_id, source_type,
                   operator_id, result, is_pass, created_at
            FROM check_results
            WHERE context_type = COALESCE(?1, context_type)
              AND context_id = COALESCE(?2, context_id)
              AND check_item_id = COALESCE(?3, check_item_id)
            ORDER BY id DESC
            "#,
        )
        .bind(filter.context_type)
        .bind(filter.context_id)
        .bind(filter.check_item_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn append_ai_execution_log(
        &self,
        input: AppendAiExecutionLogInput,
    ) -> Result<AiExecutionLog> {
        let row = sqlx::query_as::<_, AiExecutionLogRow>(
            r#"
            INSERT INTO ai_execution_logs (
              check_result_id, context_type, context_id, check_item_id,
              model_provider, model_version, temperature, prompt_snapshot, raw_output,
              input_tokens, output_tokens, exec_status, error_message, latency_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            RETURNING id, check_result_id, context_type, context_id, check_item_id,
                      model_provider, model_version, temperature, prompt_snapshot, raw_output,
                      input_tokens, output_tokens, exec_status, error_message, latency_ms,
                      created_at
            "#,
        )
        .bind(input.check_result_id)
        .bind(input.context_type)
        .bind(input.context_id)
        .bind(input.check_item_id)
        .bind(input.model_provider)
        .bind(input.model_version)
        .bind(input.temperature)
        .bind(input.prompt_snapshot)
        .bind(input.raw_output)
        .bind(input.input_tokens.unwrap_or(0))
        .bind(input.output_tokens.unwrap_or(0))
        .bind(input.exec_status.as_i64())
        .bind(input.error_message)
        .bind(input.latency_ms.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn list_ai_execution_logs(
        &self,
        filter: AiExecutionLogFilter,
    ) -> Result<Vec<AiExecutionLog>> {
        let rows = sqlx::query_as::<_, AiExecutionLogRow>(
            r#"
            SELECT id, check_result_id, context_type, context_id, check_item_id,
                   model_provider, model_version, temperature, prompt_snapshot, raw_output,
                   input_tokens, output_tokens, exec_status, error_message, latency_ms,
                   created_at
            FROM ai_execution_logs
            WHERE context_type = COALESCE(?1, context_type)
              AND context_id = COALESCE(?2, context_id)
              AND check_item_id = COALESCE(?3, check_item_id)
            ORDER BY id DESC
            "#,
        )
        .bind(filter.context_type)
        .bind(filter.context_id)
        .bind(filter.check_item_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    // ============== SOP Repository ==============

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

#[derive(Debug, FromRow)]
struct ChecklistItemRow {
    id: i64,
    name: String,
    prompt: String,
    target_level: String,
    result_schema: Option<String>,
    version: i64,
    status: String,
    created_at: String,
    updated_at: String,
    created_by: Option<i64>,
    updated_by: Option<i64>,
    deleted_at: Option<String>,
}

impl TryFrom<ChecklistItemRow> for ChecklistItem {
    type Error = PromptLabError;

    fn try_from(row: ChecklistItemRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            name: row.name,
            prompt: row.prompt,
            target_level: row.target_level.parse()?,
            result_schema: parse_json_option(row.result_schema)?,
            version: row.version,
            status: row.status.parse()?,
            created_at: row.created_at,
            updated_at: row.updated_at,
            created_by: row.created_by,
            updated_by: row.updated_by,
            deleted_at: row.deleted_at,
        })
    }
}

#[derive(Debug, FromRow)]
struct GoldenSetItemRow {
    golden_set_id: i64,
    checklist_item_id: i64,
    sort_order: i64,
    created_at: String,
}

impl From<GoldenSetItemRow> for GoldenSetItem {
    fn from(row: GoldenSetItemRow) -> Self {
        Self {
            golden_set_id: row.golden_set_id,
            checklist_item_id: row.checklist_item_id,
            sort_order: row.sort_order,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct CheckResultRow {
    id: i64,
    context_type: String,
    context_id: i64,
    check_item_id: i64,
    source_type: i64,
    operator_id: Option<String>,
    result: Option<String>,
    is_pass: i64,
    created_at: String,
}

impl TryFrom<CheckResultRow> for CheckResult {
    type Error = PromptLabError;

    fn try_from(row: CheckResultRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            context_type: row.context_type,
            context_id: row.context_id,
            check_item_id: row.check_item_id,
            source_type: SourceType::from_i64(row.source_type)?,
            operator_id: row.operator_id,
            result: parse_json_option(row.result)?,
            is_pass: row.is_pass == 1,
            created_at: row.created_at,
        })
    }
}

#[derive(Debug, FromRow)]
struct AiExecutionLogRow {
    id: i64,
    check_result_id: Option<i64>,
    context_type: String,
    context_id: i64,
    check_item_id: i64,
    model_provider: Option<String>,
    model_version: String,
    temperature: Option<f64>,
    prompt_snapshot: Option<String>,
    raw_output: Option<String>,
    input_tokens: i64,
    output_tokens: i64,
    exec_status: i64,
    error_message: Option<String>,
    latency_ms: i64,
    created_at: String,
}

impl TryFrom<AiExecutionLogRow> for AiExecutionLog {
    type Error = PromptLabError;

    fn try_from(row: AiExecutionLogRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            check_result_id: row.check_result_id,
            context_type: row.context_type,
            context_id: row.context_id,
            check_item_id: row.check_item_id,
            model_provider: row.model_provider,
            model_version: row.model_version,
            temperature: row.temperature,
            prompt_snapshot: row.prompt_snapshot,
            raw_output: row.raw_output,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            exec_status: ExecStatus::from_i64(row.exec_status)?,
            error_message: row.error_message,
            latency_ms: row.latency_ms,
            created_at: row.created_at,
        })
    }
}

fn parse_json_option(value: Option<String>) -> Result<Option<Value>> {
    value
        .map(|raw| serde_json::from_str::<Value>(&raw).map_err(PromptLabError::from))
        .transpose()
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
