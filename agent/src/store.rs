use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::{
    prompts::builtin_main_profile_prompt,
    types::{AgentProfileKind, AgentProfileRecord},
};

const AGENT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_profiles (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    tool_policy_json TEXT NOT NULL,
    model_config_json TEXT NOT NULL,
    allow_subagent_dispatch INTEGER NOT NULL,
    is_active INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

#[derive(Debug, Clone)]
pub struct AgentProfileStore {
    pool: SqlitePool,
}

impl AgentProfileStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn init_schema(&self) -> Result<()> {
        for statement in AGENT_SCHEMA.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }

            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .with_context(|| format!("execute agent schema statement: {statement}"))?;
        }

        Ok(())
    }

    pub async fn seed_builtin_profiles(&self) -> Result<()> {
        self.upsert_profile(&builtin_main_profile()).await
    }

    pub async fn upsert_profile(&self, profile: &AgentProfileRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agent_profiles (
                id, kind, display_name, description, system_prompt, tool_policy_json,
                model_config_json, allow_subagent_dispatch, is_active, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                display_name = excluded.display_name,
                description = excluded.description,
                system_prompt = excluded.system_prompt,
                tool_policy_json = excluded.tool_policy_json,
                model_config_json = excluded.model_config_json,
                allow_subagent_dispatch = excluded.allow_subagent_dispatch,
                is_active = excluded.is_active,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&profile.id)
        .bind(kind_to_str(&profile.kind))
        .bind(&profile.display_name)
        .bind(&profile.description)
        .bind(&profile.system_prompt)
        .bind(serde_json::to_string(&profile.tool_policy_json).context("serialize tool policy")?)
        .bind(serde_json::to_string(&profile.model_config_json).context("serialize model config")?)
        .bind(profile.allow_subagent_dispatch)
        .bind(profile.is_active)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("upsert agent profile")?;

        Ok(())
    }

    pub async fn get_profile(&self, profile_id: &str) -> Result<Option<AgentProfileRecord>> {
        let row = sqlx::query(
            r#"
            SELECT id, kind, display_name, description, system_prompt, tool_policy_json,
                   model_config_json, allow_subagent_dispatch, is_active, created_at, updated_at
            FROM agent_profiles
            WHERE id = ?
            "#,
        )
        .bind(profile_id)
        .fetch_optional(&self.pool)
        .await
        .context("fetch agent profile")?;

        row.map(decode_profile_row).transpose()
    }

    pub async fn list_profiles(&self) -> Result<Vec<AgentProfileRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, display_name, description, system_prompt, tool_policy_json,
                   model_config_json, allow_subagent_dispatch, is_active, created_at, updated_at
            FROM agent_profiles
            WHERE is_active = 1
            ORDER BY CASE kind
                WHEN 'BuiltinMain' THEN 0
                ELSE 1
            END, display_name ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("list agent profiles")?;

        rows.into_iter().map(decode_profile_row).collect()
    }
}

fn builtin_main_profile() -> AgentProfileRecord {
    let now = Utc::now();
    AgentProfileRecord {
        id: "builtin-main".into(),
        kind: AgentProfileKind::BuiltinMain,
        display_name: "Planner".into(),
        description: "System planning and dispatch agent".into(),
        system_prompt: builtin_main_profile_prompt().into(),
        tool_policy_json: serde_json::json!({
            "builtins": [
                "read",
                "glob",
                "grep",
                "update_plan",
                "dispatch_subagent",
                "list_subagent_dispatches",
                "get_subagent_dispatch"
            ]
        }),
        model_config_json: serde_json::Value::Null,
        allow_subagent_dispatch: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    }
}

fn decode_profile_row(row: sqlx::sqlite::SqliteRow) -> Result<AgentProfileRecord> {
    Ok(AgentProfileRecord {
        id: row.try_get("id")?,
        kind: parse_kind(&row.try_get::<String, _>("kind")?)?,
        display_name: row.try_get("display_name")?,
        description: row.try_get("description")?,
        system_prompt: row.try_get("system_prompt")?,
        tool_policy_json: serde_json::from_str(&row.try_get::<String, _>("tool_policy_json")?)
            .context("deserialize tool policy")?,
        model_config_json: serde_json::from_str(&row.try_get::<String, _>("model_config_json")?)
            .context("deserialize model config")?,
        allow_subagent_dispatch: row.try_get("allow_subagent_dispatch")?,
        is_active: row.try_get("is_active")?,
        created_at: parse_utc(&row.try_get::<String, _>("created_at")?)?,
        updated_at: parse_utc(&row.try_get::<String, _>("updated_at")?)?,
    })
}

fn kind_to_str(kind: &AgentProfileKind) -> &'static str {
    match kind {
        AgentProfileKind::BuiltinMain => "BuiltinMain",
        AgentProfileKind::CustomSubagent => "CustomSubagent",
    }
}

fn parse_kind(value: &str) -> Result<AgentProfileKind> {
    match value {
        "BuiltinMain" => Ok(AgentProfileKind::BuiltinMain),
        "CustomSubagent" => Ok(AgentProfileKind::CustomSubagent),
        other => anyhow::bail!("unknown agent profile kind: {other}"),
    }
}

fn parse_utc(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .with_context(|| format!("parse utc timestamp: {value}"))
}
