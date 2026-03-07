use crate::{TelemetryConfig, TelemetryError, TelemetryRecord};
use serde::Serialize;

pub struct ClickHouseWriter {
    client: reqwest::Client,
    config: TelemetryConfig,
}

#[derive(Serialize)]
struct ClickHouseRow<'a> {
    schema_version: u16,
    event_name: &'a str,
    event_priority: &'a str,
    session_id: &'a str,
    turn_id: &'a str,
    trace_id: &'a str,
    span_id: &'a str,
    sequence_no: u32,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: Option<u64>,
    billing_dedupe_key: Option<&'a str>,
    attributes_json: &'a serde_json::Value,
}

impl ClickHouseWriter {
    pub fn new(config: TelemetryConfig) -> Result<Self, TelemetryError> {
        Ok(Self {
            client: reqwest::Client::new(),
            config,
        })
    }

    pub async fn write_batch(&self, records: Vec<TelemetryRecord>) -> Result<(), TelemetryError> {
        if records.is_empty() {
            return Ok(());
        }

        // Validate all records first
        for record in &records {
            record.validate()?;
        }

        // Convert to ClickHouse rows
        let rows: Vec<ClickHouseRow<'_>> = records
            .iter()
            .map(|r| ClickHouseRow {
                schema_version: r.schema_version,
                event_name: &r.event_name,
                event_priority: match r.event_priority {
                    crate::EventPriority::High => "high",
                    crate::EventPriority::Low => "low",
                },
                session_id: &r.session_id,
                turn_id: &r.turn_id,
                trace_id: &r.trace_id,
                span_id: &r.span_id,
                sequence_no: r.sequence_no,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
                total_tokens: r.total_tokens,
                billing_dedupe_key: r.billing_dedupe_key.as_deref(),
                attributes_json: &r.attributes_json,
            })
            .collect();

        // Build JSONEachRow body
        let body = rows
            .into_iter()
            .map(|row| serde_json::to_string(&row))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");

        let query = format!(
            "INSERT INTO {}.{} FORMAT JSONEachRow",
            self.config.database, self.config.table
        );

        let response = self.client
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
            Err(TelemetryError::Write(format!("ClickHouse error: {} - {}", status, body)))
        }
    }
}
