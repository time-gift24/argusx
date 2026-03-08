use std::sync::Arc;

pub struct ArgusxRuntime {
    pub config: Arc<crate::AppConfig>,
    pub sqlite_pool: sqlx::SqlitePool,
    pub session_manager: session::manager::SessionManager,
    pub telemetry: Option<telemetry::TelemetryRuntime>,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

pub async fn build_runtime_from_config(config: crate::AppConfig) -> anyhow::Result<ArgusxRuntime> {
    if let Some(parent) = config.paths.sqlite.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = config.paths.log_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let logging = crate::logging::init_tracing(&config.telemetry, &config.paths.log_file).await?;

    let sqlite_url = format!("sqlite:{}?mode=rwc", config.paths.sqlite.display());
    let pool = sqlx::SqlitePool::connect(&sqlite_url).await?;
    let store = session::store::ThreadStore::new(pool.clone());
    store.init_schema().await?;

    let manager = session::manager::SessionManager::new("default-session".into(), store);
    manager.initialize().await?;

    Ok(ArgusxRuntime {
        config: Arc::new(config),
        sqlite_pool: pool,
        session_manager: manager,
        telemetry: logging.telemetry,
        _log_guard: logging.log_guard,
    })
}

pub async fn build_runtime() -> anyhow::Result<ArgusxRuntime> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    let app_home = std::path::PathBuf::from(home).join(".argusx");
    let (_, config) = crate::ensure_app_config_at(&app_home)?;
    build_runtime_from_config(config).await
}

impl ArgusxRuntime {
    pub fn shutdown(self, timeout: std::time::Duration) -> anyhow::Result<()> {
        if let Some(runtime) = self.telemetry {
            runtime.shutdown(timeout)?;
        }
        Ok(())
    }
}
