use std::path::Path;

use anyhow::Context;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::layer::SubscriberExt;

pub struct LoggingRuntime {
    pub telemetry: Option<telemetry::TelemetryRuntime>,
    pub log_guard: WorkerGuard,
}

pub async fn init_tracing(
    config: &crate::TelemetrySection,
    log_file: &Path,
) -> anyhow::Result<LoggingRuntime> {
    init_tracing_with_hooks(
        config,
        log_file,
        |telemetry_config| async move { telemetry::probe_clickhouse(&telemetry_config).await },
        telemetry::build_layer,
    )
    .await
}

async fn init_tracing_with_hooks<P, PFut, B>(
    config: &crate::TelemetrySection,
    log_file: &Path,
    probe_clickhouse: P,
    build_layer: B,
) -> anyhow::Result<LoggingRuntime>
where
    P: FnOnce(telemetry::TelemetryConfig) -> PFut,
    PFut: std::future::Future<Output = Result<(), telemetry::TelemetryError>>,
    B: FnOnce(
        telemetry::TelemetryConfig,
    ) -> Result<
        (telemetry::BoxTelemetryLayer, telemetry::TelemetryRuntime),
        telemetry::TelemetryError,
    >,
{
    let log_parent = log_file
        .parent()
        .with_context(|| format!("log file path `{}` has no parent", log_file.display()))?;
    std::fs::create_dir_all(log_parent)
        .with_context(|| format!("failed to create log directory `{}`", log_parent.display()))?;
    let file = std::fs::File::options()
        .create(true)
        .append(true)
        .open(log_file)
        .with_context(|| format!("failed to open log file `{}`", log_file.display()))?;
    let (writer, guard) = NonBlocking::new(file);

    // Tests may build more than one runtime in the same process. Tracing is
    // global, so once a subscriber exists we degrade subsequent bootstraps to
    // "logging already configured elsewhere" instead of failing startup.
    if tracing::dispatcher::has_been_set() {
        return Ok(LoggingRuntime {
            telemetry: None,
            log_guard: guard,
        });
    }

    if !config.enabled {
        install_file_logging(writer)?;
        return Ok(LoggingRuntime {
            telemetry: None,
            log_guard: guard,
        });
    }

    let telemetry_config = crate::to_telemetry_config(config.clone());
    match probe_clickhouse(telemetry_config.clone()).await {
        Ok(()) => match build_layer(telemetry_config) {
            Ok((telemetry_layer, telemetry_runtime)) => {
                // Telemetry layer MUST be added before fmt because Box<dyn Layer<Registry>>
                // only implements Layer<Registry>, not Layer<Layered<...>>.
                let subscriber = tracing_subscriber::registry().with(telemetry_layer).with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(writer)
                        .with_ansi(false),
                );
                tracing::subscriber::set_global_default(subscriber)
                    .context("failed to install tracing subscriber")?;

                Ok(LoggingRuntime {
                    telemetry: Some(telemetry_runtime),
                    log_guard: guard,
                })
            }
            Err(err) => degrade_to_file_logging(writer, guard, "build_layer", err),
        },
        Err(err) => degrade_to_file_logging(writer, guard, "probe_clickhouse", err),
    }
}

fn install_file_logging(writer: NonBlocking) -> anyhow::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false);
    let subscriber = tracing_subscriber::registry().with(fmt_layer);
    tracing::subscriber::set_global_default(subscriber)
        .context("failed to install tracing subscriber")
}

fn degrade_to_file_logging(
    writer: NonBlocking,
    guard: WorkerGuard,
    stage: &'static str,
    err: telemetry::TelemetryError,
) -> anyhow::Result<LoggingRuntime> {
    install_file_logging(writer)?;
    tracing::warn!(event_name = "telemetry_degraded", stage, error = %err);

    Ok(LoggingRuntime {
        telemetry: None,
        log_guard: guard,
    })
}

#[cfg(test)]
mod tests {
    use telemetry::TelemetryError;

    use super::init_tracing_with_hooks;

    fn telemetry_config(enabled: bool) -> crate::TelemetrySection {
        crate::TelemetrySection {
            enabled,
            clickhouse_url: "http://localhost:8123".into(),
            database: "argusx".into(),
            table: "telemetry_logs".into(),
            high_priority_batch_size: 5,
            low_priority_batch_size: 500,
            high_priority_flush_interval_ms: 1000,
            low_priority_flush_interval_ms: 30000,
            max_in_memory_events: 10000,
            max_retry_backoff_ms: 30000,
            full_logging: false,
            delta_events: false,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn init_tracing_degrades_when_layer_build_fails_after_successful_probe() {
        let temp = tempfile::tempdir().unwrap();
        let log_file = temp.path().join("argusx.log");

        let runtime = init_tracing_with_hooks(
            &telemetry_config(true),
            &log_file,
            |_| async { Ok(()) },
            |_| {
                Err::<(telemetry::BoxTelemetryLayer, telemetry::TelemetryRuntime), _>(
                    TelemetryError::Initialization("layer boom".into()),
                )
            },
        )
        .await
        .unwrap();
        drop(runtime);

        let log = std::fs::read_to_string(&log_file).unwrap();
        assert!(log.contains("telemetry_degraded"));
        assert!(log.contains("build_layer"));
        assert!(log.contains("layer boom"));
    }
}
