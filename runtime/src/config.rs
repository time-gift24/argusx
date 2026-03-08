use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub paths: PathsConfig,
    pub telemetry: TelemetrySection,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct PathsConfig {
    pub sqlite: PathBuf,
    pub log_file: PathBuf,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct TelemetrySection {
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
