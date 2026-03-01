use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub busy_timeout_ms: u64,
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "data/dev.db".to_string(),
            busy_timeout_ms: 5_000,
            max_connections: 5,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<String>,
    pub console: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
            console: true,
        }
    }
}

impl Settings {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let mut settings = if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            toml::from_str(&content)?
        } else {
            Settings::default()
        };

        settings.apply_env_overrides();
        Ok(settings)
    }

    pub fn load_default() -> anyhow::Result<Self> {
        Self::load("config/default.toml")
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(path) = std::env::var("ARGUSX_DB_PATH") {
            self.database.path = path;
        }
        if let Ok(timeout) = std::env::var("ARGUSX_DB_TIMEOUT_MS") {
            self.database.busy_timeout_ms = timeout.parse().unwrap_or(5000);
        }
        if let Ok(conns) = std::env::var("ARGUSX_DB_MAX_CONNS") {
            self.database.max_connections = conns.parse().unwrap_or(5);
        }
        if let Ok(level) = std::env::var("ARGUSX_LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(file) = std::env::var("ARGUSX_LOG_FILE") {
            self.logging.file = Some(file);
        }
        if let Ok(console) = std::env::var("ARGUSX_LOG_CONSOLE") {
            self.logging.console = console.parse().unwrap_or(true);
        }
    }
}
