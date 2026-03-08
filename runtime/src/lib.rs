#[cfg(test)]
mod tests;

mod bootstrap;
mod config;
mod logging;
mod paths;

use std::path::PathBuf;

pub use bootstrap::{ArgusxRuntime, build_runtime, build_runtime_from_config};
pub use config::{AppConfig, PathsConfig, TelemetrySection};

pub fn ensure_app_config_at(
    app_home: impl AsRef<std::path::Path>,
) -> anyhow::Result<(PathBuf, AppConfig)> {
    let app_home = app_home.as_ref();
    paths::ensure_app_home(app_home)?;
    let config_path = app_home.join("argusx.toml");

    if !config_path.exists() {
        std::fs::write(&config_path, default_config_toml())?;
    }

    let raw = std::fs::read_to_string(&config_path)?;
    let mut config: AppConfig = toml::from_str(&raw)?;
    config.paths.sqlite = paths::resolve_path(&config.paths.sqlite, app_home);
    config.paths.log_file = paths::resolve_path(&config.paths.log_file, app_home);

    Ok((config_path, config))
}

fn default_config_toml() -> &'static str {
    r#"[paths]
sqlite = "~/.argusx/sqlite.db"
log_file = "~/.argusx/argusx.log"

[telemetry]
enabled = true
clickhouse_url = "http://localhost:8123"
database = "argusx"
table = "telemetry_logs"
high_priority_batch_size = 5
low_priority_batch_size = 500
high_priority_flush_interval_ms = 1000
low_priority_flush_interval_ms = 30000
max_in_memory_events = 10000
max_retry_backoff_ms = 30000
full_logging = false
delta_events = false
"#
}

pub(crate) fn to_telemetry_config(section: TelemetrySection) -> telemetry::TelemetryConfig {
    telemetry::TelemetryConfig {
        enabled: section.enabled,
        clickhouse_url: section.clickhouse_url,
        database: section.database,
        table: section.table,
        high_priority_batch_size: section.high_priority_batch_size,
        low_priority_batch_size: section.low_priority_batch_size,
        high_priority_flush_interval_ms: section.high_priority_flush_interval_ms,
        low_priority_flush_interval_ms: section.low_priority_flush_interval_ms,
        max_in_memory_events: section.max_in_memory_events,
        max_retry_backoff_ms: section.max_retry_backoff_ms,
        full_logging: section.full_logging,
        delta_events: section.delta_events,
    }
}
