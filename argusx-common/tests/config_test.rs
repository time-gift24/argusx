use argusx_common::config::{DatabaseConfig, LoggingConfig, Settings};
use std::fs;
use tempfile::TempDir;

fn cleanup_env_vars() {
    // Clean all possible env vars - ignore errors if they're not set
    std::env::remove_var("ARGUSX_DB_PATH");
    std::env::remove_var("ARGUSX_LOG_LEVEL");
    std::env::remove_var("ARGUSX_DB_TIMEOUT_MS");
    std::env::remove_var("ARGUSX_DB_MAX_CONNS");
    std::env::remove_var("ARGUSX_LOG_FILE");
    std::env::remove_var("ARGUSX_LOG_CONSOLE");
}

fn cleanup_after_test() {
    // Cleanup after test - this is called in a Drop impl or manually
    cleanup_env_vars();
}

#[test]
fn test_settings_default() {
    cleanup_env_vars();
    let settings = Settings::default();
    assert_eq!(settings.database.path, "data/dev.db");
    assert_eq!(settings.database.busy_timeout_ms, 5000);
    assert_eq!(settings.database.max_connections, 5);
    assert_eq!(settings.logging.level, "info");
    assert!(settings.logging.console);
    assert!(settings.logging.file.is_none());
    cleanup_after_test();
}

#[test]
fn test_settings_load_from_file() {
    cleanup_env_vars();
    let _ = std::env::var("ARGUSX_DB_PATH").ok(); // This ensures remove_var is called
    let _ = std::env::var("ARGUSX_LOG_LEVEL").ok();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_content = r#"
[database]
path = "test.db"
busy_timeout_ms = 1000
max_connections = 10

[logging]
level = "debug"
console = false
"#;
    fs::write(&config_path, config_content).unwrap();

    let settings = Settings::load(config_path.to_str().unwrap()).unwrap();

    assert_eq!(settings.database.path, "test.db");
    assert_eq!(settings.database.busy_timeout_ms, 1000);
    assert_eq!(settings.database.max_connections, 10);
    assert_eq!(settings.logging.level, "debug");
    assert!(!settings.logging.console);
    cleanup_after_test();
}

#[test]
fn test_database_config_default() {
    cleanup_env_vars();
    let config = DatabaseConfig::default();
    assert_eq!(config.path, "data/dev.db");
    assert_eq!(config.busy_timeout_ms, 5000);
    assert_eq!(config.max_connections, 5);
    cleanup_after_test();
}

#[test]
fn test_logging_config_default() {
    cleanup_env_vars();
    let config = LoggingConfig::default();
    assert_eq!(config.level, "info");
    assert!(config.console);
    assert!(config.file.is_none());
    cleanup_after_test();
}
