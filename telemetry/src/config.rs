#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub clickhouse_url: String,
    pub database: String,
    pub table: String,
    pub high_priority_batch_size: usize,
    pub low_priority_batch_size: usize,
    pub high_priority_flush_interval_ms: u64,
    pub low_priority_flush_interval_ms: u64,
    pub max_in_memory_events: usize,
    pub max_retry_backoff_ms: u64,
    pub full_logging: bool,
    pub delta_events: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            clickhouse_url: "http://localhost:8123".to_string(),
            database: "argusx".to_string(),
            table: "telemetry_logs".to_string(),
            high_priority_batch_size: 5,
            low_priority_batch_size: 500,
            high_priority_flush_interval_ms: 1_000,
            low_priority_flush_interval_ms: 30_000,
            max_in_memory_events: 10_000,
            max_retry_backoff_ms: 30_000,
            full_logging: false,
            delta_events: false,
        }
    }
}
