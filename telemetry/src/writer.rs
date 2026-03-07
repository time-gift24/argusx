use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::{TelemetryConfig, TelemetryError, TelemetryRecord};

#[async_trait]
pub trait BatchWriter: Send + Sync + 'static {
    async fn write_batch(&self, records: Vec<TelemetryRecord>) -> Result<(), TelemetryError>;
}

pub struct ClickHouseWriter {
    client: reqwest::Client,
    config: TelemetryConfig,
}

#[derive(Serialize)]
struct ClickHouseRow<'a> {
    ingest_id: String,
    schema_version: u16,
    occurred_at: String,
    trace_id: &'a str,
    span_id: &'a str,
    parent_span_id: Option<&'a str>,
    session_id: &'a str,
    turn_id: &'a str,
    step_index: Option<u32>,
    sequence_no: u32,
    level: &'a str,
    target: &'a str,
    event_name: &'a str,
    event_priority: &'a str,
    user_id: Option<&'a str>,
    model_name: Option<&'a str>,
    provider: Option<&'a str>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: Option<u64>,
    billing_dedupe_key: Option<&'a str>,
    tool_name: Option<&'a str>,
    tool_outcome: Option<&'a str>,
    tool_duration_ms: Option<u64>,
    error_code: Option<&'a str>,
    error_message: Option<&'a str>,
    request_preview: Option<&'a str>,
    response_preview: Option<&'a str>,
    attributes_json: String,
}

impl ClickHouseWriter {
    pub fn new(config: TelemetryConfig) -> Result<Self, TelemetryError> {
        Ok(Self {
            client: reqwest::Client::new(),
            config,
        })
    }

    pub async fn write_batch(&self, records: Vec<TelemetryRecord>) -> Result<(), TelemetryError> {
        <Self as BatchWriter>::write_batch(self, records).await
    }
}

#[async_trait]
impl BatchWriter for ClickHouseWriter {
    async fn write_batch(&self, records: Vec<TelemetryRecord>) -> Result<(), TelemetryError> {
        if records.is_empty() {
            return Ok(());
        }

        for record in &records {
            record.validate()?;
        }

        let rows: Vec<ClickHouseRow<'_>> = records
            .iter()
            .map(|record| {
                Ok(ClickHouseRow {
                    ingest_id: record
                        .ingest_id
                        .unwrap_or_else(Uuid::new_v4)
                        .hyphenated()
                        .to_string(),
                    schema_version: record.schema_version,
                    occurred_at: record
                        .occurred_at
                        .format("%Y-%m-%d %H:%M:%S%.3f")
                        .to_string(),
                    trace_id: &record.trace_id,
                    span_id: &record.span_id,
                    parent_span_id: record.parent_span_id.as_deref(),
                    session_id: &record.session_id,
                    turn_id: &record.turn_id,
                    step_index: record.step_index,
                    sequence_no: record.sequence_no,
                    level: &record.level,
                    target: &record.target,
                    event_name: &record.event_name,
                    event_priority: match record.event_priority {
                        crate::EventPriority::High => "high",
                        crate::EventPriority::Low => "low",
                    },
                    user_id: record.user_id.as_deref(),
                    model_name: record.model_name.as_deref(),
                    provider: record.provider.as_deref(),
                    input_tokens: record.input_tokens,
                    output_tokens: record.output_tokens,
                    total_tokens: record.total_tokens,
                    billing_dedupe_key: record.billing_dedupe_key.as_deref(),
                    tool_name: record.tool_name.as_deref(),
                    tool_outcome: record.tool_outcome.as_deref(),
                    tool_duration_ms: record.tool_duration_ms,
                    error_code: record.error_code.as_deref(),
                    error_message: record.error_message.as_deref(),
                    request_preview: record.request_preview.as_deref(),
                    response_preview: record.response_preview.as_deref(),
                    attributes_json: serde_json::to_string(&record.attributes_json)?,
                })
            })
            .collect::<Result<_, serde_json::Error>>()?;

        let body = rows
            .into_iter()
            .map(|row| serde_json::to_string(&row))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");

        let query = format!(
            "INSERT INTO {}.{} FORMAT JSONEachRow",
            self.config.database, self.config.table
        );

        let response = self
            .client
            .post(&self.config.clickhouse_url)
            .query(&[("query", query)])
            .header("Content-Type", "application/x-ndjson")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(TelemetryError::Write(format!(
                "ClickHouse error: {} - {}",
                status, body
            )))
        }
    }
}
