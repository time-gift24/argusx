use anyhow::Context;

use std::sync::Arc;

pub struct ArgusxRuntime {
    pub config: Arc<crate::AppConfig>,
    pub sqlite_pool: sqlx::SqlitePool,
    pub session_manager: session::manager::SessionManager,
    pub agent_profiles: Arc<agent::AgentProfileStore>,
    pub agent_execution_resolver: Arc<agent::AgentExecutionResolver>,
    pub telemetry: Option<telemetry::TelemetryRuntime>,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

pub async fn build_runtime_from_config(config: crate::AppConfig) -> anyhow::Result<ArgusxRuntime> {
    if let Some(parent) = config.paths.sqlite.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create sqlite directory `{}`", parent.display()))?;
    }
    if let Some(parent) = config.paths.log_file.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory `{}`", parent.display()))?;
    }

    let logging = crate::logging::init_tracing(&config.telemetry, &config.paths.log_file)
        .await
        .with_context(|| {
            format!(
                "failed to initialize tracing with log file `{}`",
                config.paths.log_file.display()
            )
        })?;

    let sqlite_url = format!("sqlite:{}?mode=rwc", config.paths.sqlite.display());
    let pool = sqlx::SqlitePool::connect(&sqlite_url)
        .await
        .with_context(|| {
            format!(
                "failed to connect sqlite at `{}`",
                config.paths.sqlite.display()
            )
        })
        .map_err(|err| {
            tracing::error!(event_name = "runtime_bootstrap_failed", stage = "sqlite_connect", error = %err);
            err
        })?;
    let agent_profiles = Arc::new(agent::AgentProfileStore::new(pool.clone()));
    agent_profiles
        .init_schema()
        .await
        .context("failed to initialize agent schema")
        .map_err(|err| {
            tracing::error!(event_name = "runtime_bootstrap_failed", stage = "agent_init_schema", error = %err);
            err
        })?;
    agent_profiles
        .seed_builtin_profiles()
        .await
        .context("failed to seed builtin agent profiles")
        .map_err(|err| {
            tracing::error!(event_name = "runtime_bootstrap_failed", stage = "agent_seed_builtin_profiles", error = %err);
            err
        })?;
    let store = session::store::ThreadStore::new(pool.clone());
    store
        .init_schema()
        .await
        .context("failed to initialize session schema")
        .map_err(|err| {
            tracing::error!(event_name = "runtime_bootstrap_failed", stage = "init_schema", error = %err);
            err
        })?;

    let manager = session::manager::SessionManager::new("default-session".into(), store);
    manager
        .initialize()
        .await
        .context("failed to initialize session manager")
        .map_err(|err| {
            tracing::error!(event_name = "runtime_bootstrap_failed", stage = "session_initialize", error = %err);
            err
        })?;

    Ok(ArgusxRuntime {
        config: Arc::new(config),
        sqlite_pool: pool,
        session_manager: manager,
        agent_profiles,
        agent_execution_resolver: Arc::new(agent::AgentExecutionResolver::new()),
        telemetry: logging.telemetry,
        _log_guard: logging.log_guard,
    })
}

pub async fn build_runtime() -> anyhow::Result<ArgusxRuntime> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    let app_home = std::path::PathBuf::from(home).join(".argusx");
    let (_, config) = crate::ensure_app_config_at(&app_home).with_context(|| {
        format!(
            "failed to load runtime config from `{}`",
            app_home.display()
        )
    })?;
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
