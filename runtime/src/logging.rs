use std::path::Path;

use tracing_appender::non_blocking::{WorkerGuard, NonBlocking};
use tracing_subscriber::layer::SubscriberExt;

pub struct LoggingRuntime {
    pub telemetry: Option<telemetry::TelemetryRuntime>,
    pub log_guard: WorkerGuard,
}

pub async fn init_tracing(
    config: &crate::TelemetrySection,
    log_file: &Path,
) -> anyhow::Result<LoggingRuntime> {
    let log_parent = log_file.parent().expect("log file has parent");
    std::fs::create_dir_all(log_parent)?;
    let file = std::fs::File::options().create(true).append(true).open(log_file)?;
    let (writer, guard) = NonBlocking::new(file);

    if config.enabled {
        let telemetry_config = crate::to_telemetry_config(config.clone());
        match telemetry::probe_clickhouse(&telemetry_config).await {
            Ok(()) => {
                // Build telemetry layer first (adds to Registry)
                let (telemetry_layer, telemetry_runtime) = telemetry::build_layer(telemetry_config)?;

                // Add telemetry layer to Registry, then fmt layer
                // IMPORTANT: Telemetry layer MUST be added first because Box<dyn Layer<Registry>>
                // only implements Layer<Registry>, not Layer<Layered<...>>
                let subscriber = tracing_subscriber::registry()
                    .with(telemetry_layer)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_writer(writer)
                            .with_ansi(false)
                    );
                tracing::subscriber::set_global_default(subscriber)?;

                Ok(LoggingRuntime {
                    telemetry: Some(telemetry_runtime),
                    log_guard: guard,
                })
            }
            Err(err) => {
                // Telemetry unavailable - just use file logging
                let fmt_layer = tracing_subscriber::fmt::layer()
                    .with_writer(writer)
                    .with_ansi(false);
                let subscriber = tracing_subscriber::registry().with(fmt_layer);
                tracing::subscriber::set_global_default(subscriber)?;
                tracing::warn!(event_name = "telemetry_degraded", error = %err);

                Ok(LoggingRuntime {
                    telemetry: None,
                    log_guard: guard,
                })
            }
        }
    } else {
        // Telemetry disabled - just use file logging
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_ansi(false);
        let subscriber = tracing_subscriber::registry().with(fmt_layer);
        tracing::subscriber::set_global_default(subscriber)?;

        Ok(LoggingRuntime {
            telemetry: None,
            log_guard: guard,
        })
    }
}
