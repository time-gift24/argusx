# Runtime Bootstrap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a reusable `runtime` crate that owns config/bootstrap/logging/telemetry/SQLite/session initialization, then switch desktop to consume it instead of hardcoding startup logic.

**Architecture:** Follow [runtime bootstrap design](/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/docs/plans/2026-03-08-runtime-bootstrap-design.md). `session` remains the owner of `SessionManager` and persistence logic, `runtime` becomes the startup/resource owner, and `desktop` becomes a thin Tauri bridge. Telemetry must be composed into a runtime-owned tracing subscriber, and startup must degrade cleanly when ClickHouse probing fails.

**Tech Stack:** Rust 2024, Tokio, sqlx/sqlite, tracing, tracing-subscriber, tracing-appender, serde/toml, Tauri 2

---

**Reference skills:** @rust-router @m12-lifecycle @test-driven-development @verification-before-completion

### Task 1: Create the `runtime` crate skeleton

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/Cargo.toml`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/Cargo.toml`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/lib.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/tests.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/tests.rs`

**Step 1: Write the failing smoke test**

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/tests.rs`:

```rust
use crate::{AppConfig, ArgusxRuntime, build_runtime};

#[test]
fn runtime_crate_exports_bootstrap_surface() {
    let _ = std::any::type_name::<AppConfig>();
    let _ = std::any::type_name::<ArgusxRuntime>();
    let _ = build_runtime;
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p runtime runtime_crate_exports_bootstrap_surface -- --exact`

Expected: FAIL because the `runtime` package does not exist in the workspace yet.

**Step 3: Add the crate and the minimal public surface**

Modify `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/Cargo.toml`:

```toml
[workspace]
members = ["core", "desktop/src-tauri", "provider", "runtime", "session", "tool", "turn", "telemetry", "vendor/eventsource_stream"]
```

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/Cargo.toml`:

```toml
[package]
name = "runtime"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = { workspace = true }
serde = { workspace = true, features = ["derive"] }
session = { path = "../session" }
sqlx = { workspace = true, features = ["runtime-tokio", "sqlite"] }
telemetry = { path = "../telemetry" }
tokio = { workspace = true, features = ["fs", "rt-multi-thread", "macros"] }
toml = { workspace = true }
tracing = { workspace = true }
tracing-appender = { workspace = true }
tracing-subscriber = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/lib.rs`:

```rust
mod tests;

#[derive(Debug, Clone)]
pub struct AppConfig;

pub struct ArgusxRuntime;

pub async fn build_runtime() -> anyhow::Result<ArgusxRuntime> {
    anyhow::bail!("not implemented")
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p runtime runtime_crate_exports_bootstrap_surface -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml runtime/Cargo.toml runtime/src/lib.rs runtime/src/tests.rs
git commit -m "feat(runtime): add bootstrap crate skeleton"
```

### Task 2: Implement config bootstrap and path resolution

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/lib.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/config.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/paths.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/config_bootstrap_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/config_bootstrap_test.rs`

**Step 1: Write the failing tests**

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/config_bootstrap_test.rs`:

```rust
use runtime::{ensure_app_config_at, AppConfig};

#[test]
fn ensure_app_config_creates_default_file_and_expands_paths() {
    let temp = tempfile::tempdir().unwrap();
    let app_home = temp.path().join(".argusx");

    let (config_path, config) = ensure_app_config_at(&app_home).unwrap();

    assert_eq!(config_path, app_home.join("argusx.toml"));
    assert!(config_path.exists());
    assert_eq!(config.paths.sqlite, app_home.join("sqlite.db"));
    assert_eq!(config.paths.log_file, app_home.join("argusx.log"));
}

#[test]
fn ensure_app_config_reuses_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let app_home = temp.path().join(".argusx");
    std::fs::create_dir_all(&app_home).unwrap();
    std::fs::write(
        app_home.join("argusx.toml"),
        r#"
[paths]
sqlite = "./state/app.db"
log_file = "./logs/app.log"

[telemetry]
enabled = false
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
"#,
    )
    .unwrap();

    let (_, config) = ensure_app_config_at(&app_home).unwrap();

    assert_eq!(config.paths.sqlite, app_home.join("state/app.db"));
    assert_eq!(config.paths.log_file, app_home.join("logs/app.log"));
    assert!(!config.telemetry.enabled);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p runtime --test config_bootstrap_test -- --nocapture`

Expected: FAIL with unresolved import `ensure_app_config_at` or missing `paths` fields on `AppConfig`.

**Step 3: Implement the config model and bootstrap helpers**

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/config.rs`:

```rust
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
```

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/paths.rs`:

```rust
use std::path::{Path, PathBuf};

pub fn ensure_app_home(app_home: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_home)?;
    Ok(())
}

pub fn resolve_path(raw: &Path, app_home: &Path) -> PathBuf {
    let raw_str = raw.to_string_lossy();
    if raw_str == "~/.argusx/sqlite.db" {
        return app_home.join("sqlite.db");
    }
    if raw_str == "~/.argusx/argusx.log" {
        return app_home.join("argusx.log");
    }
    if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        app_home.join(raw)
    }
}
```

Update `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/lib.rs`:

```rust
mod config;
mod paths;

pub use config::{AppConfig, PathsConfig, TelemetrySection};

pub fn ensure_app_config_at(app_home: impl AsRef<std::path::Path>) -> anyhow::Result<(std::path::PathBuf, AppConfig)> {
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p runtime --test config_bootstrap_test -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add runtime/src/lib.rs runtime/src/config.rs runtime/src/paths.rs runtime/tests/config_bootstrap_test.rs
git commit -m "feat(runtime): add unified config bootstrap"
```

### Task 3: Refactor telemetry for startup probing and external subscriber composition

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/src/lib.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/src/runtime.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/src/writer.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/probe_clickhouse_test.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/runtime_integration_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/probe_clickhouse_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/runtime_integration_test.rs`

**Step 1: Write the failing tests**

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/probe_clickhouse_test.rs`:

```rust
use telemetry::{probe_clickhouse, TelemetryConfig};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn probe_succeeds_against_healthy_clickhouse() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(query_param("query", "SELECT 1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .mount(&server)
        .await;

    let config = TelemetryConfig {
        clickhouse_url: server.uri(),
        ..TelemetryConfig::default()
    };

    probe_clickhouse(&config).await.unwrap();
}

#[tokio::test]
async fn probe_fails_on_non_success_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let config = TelemetryConfig {
        clickhouse_url: server.uri(),
        ..TelemetryConfig::default()
    };

    let err = probe_clickhouse(&config).await.unwrap_err();
    assert!(err.to_string().contains("503"));
}
```

Update `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/runtime_integration_test.rs` to expect external composition:

```rust
use std::time::Duration;

use telemetry::{build_layer, TelemetryConfig};
use tracing_subscriber::layer::SubscriberExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_flushes_high_priority_records_with_external_subscriber() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = TelemetryConfig {
        clickhouse_url: server.uri(),
        high_priority_batch_size: 100,
        low_priority_batch_size: 100,
        high_priority_flush_interval_ms: 60_000,
        low_priority_flush_interval_ms: 60_000,
        ..TelemetryConfig::default()
    };

    let (layer, runtime) = build_layer(config).unwrap();
    let subscriber = tracing_subscriber::registry().with(layer);
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::info!(
        event_name = "turn_finished",
        event_priority = "high",
        session_id = "session-1",
        turn_id = "turn-1",
        sequence_no = 1u64
    );

    runtime.shutdown(Duration::from_secs(2)).unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p telemetry --test probe_clickhouse_test -- --nocapture`

Expected: FAIL with unresolved import `probe_clickhouse`.

Run: `cargo test -p telemetry --test runtime_integration_test -- --nocapture`

Expected: FAIL with unresolved import `build_layer`.

**Step 3: Implement the telemetry probe and composable layer API**

Modify `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/src/writer.rs`:

```rust
impl ClickHouseWriter {
    pub async fn probe(&self) -> Result<(), TelemetryError> {
        let response = self
            .client
            .post(&self.config.clickhouse_url)
            .query(&[("query", "SELECT 1")])
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(TelemetryError::Write(format!(
                "ClickHouse probe failed: {} - {}",
                status, body
            )))
        }
    }
}

pub async fn probe_clickhouse(config: &TelemetryConfig) -> Result<(), TelemetryError> {
    ClickHouseWriter::new(config.clone())?.probe().await
}
```

Modify `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/src/runtime.rs`:

```rust
pub type BoxTelemetryLayer =
    Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>;

pub fn build_layer(config: TelemetryConfig) -> Result<(BoxTelemetryLayer, TelemetryRuntime), TelemetryError> {
    let writer = Arc::new(ClickHouseWriter::new(config.clone())?);
    build_layer_with_writer(config, writer)
}

fn build_layer_with_writer(
    config: TelemetryConfig,
    writer: Arc<dyn BatchWriter>,
) -> Result<(BoxTelemetryLayer, TelemetryRuntime), TelemetryError> {
    let queue = Arc::new(Mutex::new(BatchQueue::new(config.clone())));
    let notify = Arc::new(tokio::sync::Notify::new());
    let metrics = Arc::new(TelemetryMetricsInner::default());
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let (shutdown_complete_tx, shutdown_complete_rx) = std_mpsc::channel();
    let writer_metrics = metrics.clone();

    let layer = TelemetryLayer::new(
        RuntimeSink::new(queue.clone(), notify.clone(), metrics.clone()),
        config.clone(),
    );

    std::thread::Builder::new()
        .name("telemetry-writer".to_string())
        .spawn(move || {
            // existing writer task setup
        })?;

    let runtime = TelemetryRuntime {
        shutdown_tx: Some(shutdown_tx),
        shutdown_complete_rx,
        metrics,
    };

    Ok((Box::new(layer), runtime))
}

pub fn init(config: TelemetryConfig) -> Result<TelemetryRuntime, TelemetryError> {
    let (layer, runtime) = build_layer(config)?;
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|err| TelemetryError::Initialization(err.to_string()))?;
    Ok(runtime)
}
```

Modify `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/src/lib.rs`:

```rust
pub use runtime::{BoxTelemetryLayer, DegradationPolicy, TelemetryMetrics, TelemetryRuntime, build_layer, init};
pub use writer::{ClickHouseWriter, probe_clickhouse};
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p telemetry --test probe_clickhouse_test -- --nocapture`

Expected: PASS

Run: `cargo test -p telemetry --test runtime_integration_test -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add telemetry/src/lib.rs telemetry/src/runtime.rs telemetry/src/writer.rs telemetry/tests/probe_clickhouse_test.rs telemetry/tests/runtime_integration_test.rs
git commit -m "feat(telemetry): add startup probe and composable layer API"
```

### Task 4: Implement runtime logging, telemetry degradation, SQLite boot, and shutdown

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/Cargo.toml`
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/lib.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/logging.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/bootstrap.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_build_test.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_degraded_telemetry_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_build_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_degraded_telemetry_test.rs`

**Step 1: Write the failing tests**

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_build_test.rs`:

```rust
use runtime::{build_runtime_from_config, AppConfig, PathsConfig, TelemetrySection};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn build_runtime_with_disabled_telemetry_creates_sqlite_and_initializes_session() {
    let temp = tempfile::tempdir().unwrap();
    let sqlite = temp.path().join("sqlite.db");
    let log_file = temp.path().join("argusx.log");

    let config = AppConfig {
        paths: PathsConfig {
            sqlite: sqlite.clone(),
            log_file: log_file.clone(),
        },
        telemetry: TelemetrySection {
            enabled: false,
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
        },
    };

    let runtime = build_runtime_from_config(config).await.unwrap();

    assert!(sqlite.exists());
    assert!(log_file.exists());
    assert!(runtime.telemetry.is_none());
    assert!(runtime.session_manager.list_threads().await.unwrap().is_empty());
}
```

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_degraded_telemetry_test.rs`:

```rust
use runtime::{build_runtime_from_config, AppConfig, PathsConfig, TelemetrySection};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn build_runtime_degrades_when_clickhouse_probe_fails() {
    let temp = tempfile::tempdir().unwrap();
    let config = AppConfig {
        paths: PathsConfig {
            sqlite: temp.path().join("sqlite.db"),
            log_file: temp.path().join("argusx.log"),
        },
        telemetry: TelemetrySection {
            enabled: true,
            clickhouse_url: "http://127.0.0.1:9".into(),
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
        },
    };

    let runtime = build_runtime_from_config(config).await.unwrap();

    assert!(runtime.telemetry.is_none());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p runtime --test runtime_build_test -- --nocapture`

Expected: FAIL with unresolved import `build_runtime_from_config`.

Run: `cargo test -p runtime --test runtime_degraded_telemetry_test -- --nocapture`

Expected: FAIL with unresolved import `build_runtime_from_config`.

**Step 3: Implement logging/bootstrap/runtime ownership**

Update `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/Cargo.toml`:

```toml
[dependencies]
anyhow = { workspace = true }
serde = { workspace = true, features = ["derive"] }
session = { path = "../session" }
sqlx = { workspace = true, features = ["runtime-tokio", "sqlite"] }
telemetry = { path = "../telemetry" }
tokio = { workspace = true, features = ["fs", "rt-multi-thread", "macros"] }
toml = { workspace = true }
tracing = { workspace = true }
tracing-appender = { workspace = true }
tracing-subscriber = { workspace = true }
```

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/logging.rs`:

```rust
use std::path::Path;

use tracing_appender::non_blocking::{WorkerGuard, non_blocking};
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
    let (writer, guard) = non_blocking(file);
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(writer).with_ansi(false);

    let telemetry = if config.enabled {
        let telemetry_config = crate::to_telemetry_config(config.clone());
        match telemetry::probe_clickhouse(&telemetry_config).await {
            Ok(()) => {
                let (telemetry_layer, telemetry_runtime) = telemetry::build_layer(telemetry_config)?;
                let subscriber = tracing_subscriber::registry().with(fmt_layer).with(telemetry_layer);
                tracing::subscriber::set_global_default(subscriber)?;
                Some(telemetry_runtime)
            }
            Err(err) => {
                let subscriber = tracing_subscriber::registry().with(fmt_layer);
                tracing::subscriber::set_global_default(subscriber)?;
                tracing::warn!(event_name = "telemetry_degraded", error = %err);
                None
            }
        }
    } else {
        let subscriber = tracing_subscriber::registry().with(fmt_layer);
        tracing::subscriber::set_global_default(subscriber)?;
        None
    };

    Ok(LoggingRuntime {
        telemetry,
        log_guard: guard,
    })
}
```

Create `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/bootstrap.rs`:

```rust
pub struct ArgusxRuntime {
    pub config: std::sync::Arc<crate::AppConfig>,
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

    let sqlite_url = format!("sqlite:{}", config.paths.sqlite.display());
    let pool = sqlx::SqlitePool::connect(&sqlite_url).await?;
    let store = session::store::ThreadStore::new(pool.clone());
    store.init_schema().await?;

    let manager = session::manager::SessionManager::new("default-session".into(), store);
    manager.initialize().await?;

    Ok(ArgusxRuntime {
        config: std::sync::Arc::new(config),
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
```

Update `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/src/lib.rs`:

```rust
mod bootstrap;
mod config;
mod logging;
mod paths;

pub use bootstrap::{ArgusxRuntime, build_runtime, build_runtime_from_config};
pub use config::{AppConfig, PathsConfig, TelemetrySection};

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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p runtime --test runtime_build_test -- --nocapture`

Expected: PASS

Run: `cargo test -p runtime --test runtime_degraded_telemetry_test -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add runtime/Cargo.toml runtime/src/lib.rs runtime/src/logging.rs runtime/src/bootstrap.rs runtime/tests/runtime_build_test.rs runtime/tests/runtime_degraded_telemetry_test.rs
git commit -m "feat(runtime): bootstrap logging telemetry and session startup"
```

### Task 5: Switch desktop to consume `runtime`

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/Cargo.toml`
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/src/lib.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/src/session_commands.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/src/lib.rs`

**Step 1: Write the failing smoke test**

Update `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/src/lib.rs` tests:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn desktop_builds_against_runtime_crate() {
        let _ = std::any::type_name::<runtime::ArgusxRuntime>();
    }
}
```

**Step 2: Run the desktop check to verify integration is still missing**

Run: `cargo check -p desktop`

Expected: FAIL once you add the `runtime` dependency but before replacing direct telemetry/SQLite/session bootstrap calls.

**Step 3: Replace desktop-owned startup with runtime-owned startup**

Modify `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/Cargo.toml`:

```toml
[dependencies]
runtime = { path = "../../runtime" }
session = { path = "../../session" }
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "sync"] }
tracing = { workspace = true }
uuid = { workspace = true, features = ["v4"] }
```

Modify `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/src/lib.rs`:

```rust
mod session_commands;

use session_commands::{
    DesktopSessionState, cancel_thread_turn, create_thread, list_threads,
    resolve_thread_permission, send_message, spawn_session_event_bridge, switch_thread,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tauri::async_runtime::block_on(runtime::build_runtime())?;
    let manager = runtime.session_manager.clone();
    let session_state = DesktopSessionState::new(manager);
    let bridge_manager = session_state.manager.clone();

    let run_result = tauri::Builder::default()
        .manage(session_state)
        .setup(move |app| {
            spawn_session_event_bridge(app.handle().clone(), bridge_manager.clone());
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            create_thread,
            list_threads,
            switch_thread,
            send_message,
            resolve_thread_permission,
            cancel_thread_turn,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = run_result {
        runtime.shutdown(std::time::Duration::from_secs(10))?;
        return Err(Box::new(err));
    }

    runtime.shutdown(std::time::Duration::from_secs(10))?;
    Ok(())
}
```

Modify `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/desktop/src-tauri/src/session_commands.rs` only as needed to keep `DesktopSessionState` owning a manager wrapper, not a runtime builder.

**Step 4: Run verification**

Run: `cargo check -p desktop`

Expected: PASS

Run: `cargo test -p runtime --test config_bootstrap_test -- --nocapture`

Expected: PASS

Run: `cargo test -p runtime --test runtime_build_test -- --nocapture`

Expected: PASS

Run: `cargo test -p runtime --test runtime_degraded_telemetry_test -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/Cargo.toml desktop/src-tauri/src/lib.rs desktop/src-tauri/src/session_commands.rs
git commit -m "feat(desktop): consume shared runtime bootstrap"
```

### Task 6: Final regression verification

**Files:**
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/config_bootstrap_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_build_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/runtime/tests/runtime_degraded_telemetry_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/probe_clickhouse_test.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/.worktrees/session-thread-turn-v2/telemetry/tests/runtime_integration_test.rs`

**Step 1: Run the runtime and telemetry test sweep**

Run:

```bash
cargo test -p telemetry --test probe_clickhouse_test -- --nocapture
cargo test -p telemetry --test runtime_integration_test -- --nocapture
cargo test -p runtime --test config_bootstrap_test -- --nocapture
cargo test -p runtime --test runtime_build_test -- --nocapture
cargo test -p runtime --test runtime_degraded_telemetry_test -- --nocapture
cargo check -p desktop
```

Expected:

- All tests PASS
- `cargo check -p desktop` PASS

**Step 2: Run a manual startup smoke test**

Run:

```bash
rm -rf ~/.argusx
cargo check -p desktop
```

Expected:

- The codebase compiles with the new startup path
- On first real app launch, `~/.argusx/argusx.toml` would be auto-created

**Step 3: Commit the final verified batch**

```bash
git add Cargo.toml runtime telemetry desktop
git commit -m "feat(runtime): unify bootstrap for config logging and session startup"
```
