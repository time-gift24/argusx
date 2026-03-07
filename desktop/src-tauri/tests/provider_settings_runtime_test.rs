use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use desktop_lib::{
    chat::ProviderModelRunner,
    provider_settings::{
        AesGcmSecretBox, DataKeyStore, ProviderProfileStore, ProviderSettingsError,
        ProviderSettingsService, SaveProviderProfileInput,
    },
};
use uuid::Uuid;

#[test]
fn provider_model_runner_uses_sqlite_default_profile_when_env_is_missing() {
    let _guard = env_lock().lock().unwrap();
    clear_provider_env();

    let db_path = temp_db_path("provider-settings-runtime-sqlite");
    let service = test_service(&db_path);
    service
        .save_profile(SaveProviderProfileInput {
            id: None,
            name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1/".into(),
            model: "openai/gpt-4.1-mini".into(),
            api_key: Some("sk-sqlite".into()),
            is_default: true,
        })
        .unwrap();

    assert!(ProviderModelRunner::from_provider_settings(Some(&service)).is_ok());
}

#[test]
fn provider_model_runner_falls_back_to_env_when_sqlite_has_no_default_profile() {
    let _guard = env_lock().lock().unwrap();
    clear_provider_env();
    std::env::set_var("ARGUSX_MODEL", "gpt-4.1-mini");
    std::env::set_var("ARGUSX_PROVIDER_BASE_URL", "https://api.openai.com/v1/");
    std::env::set_var("ARGUSX_PROVIDER_API_KEY", "sk-env");

    let db_path = temp_db_path("provider-settings-runtime-env");
    let service = test_service(&db_path);

    assert!(ProviderModelRunner::from_provider_settings(Some(&service)).is_ok());
    clear_provider_env();
}

struct FixedKeyStore;

impl DataKeyStore for FixedKeyStore {
    fn load_or_create_key(&self) -> Result<[u8; 32], ProviderSettingsError> {
        Ok([11; 32])
    }
}

fn test_service(db_path: &PathBuf) -> ProviderSettingsService {
    let store = ProviderProfileStore::new(db_path).unwrap();
    let secret_box = AesGcmSecretBox::new(Arc::new(FixedKeyStore));
    ProviderSettingsService::new(store, secret_box)
}

fn temp_db_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.db", Uuid::new_v4()))
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn clear_provider_env() {
    for key in [
        "ARGUSX_MODEL",
        "ARGUSX_PROVIDER_DIALECT",
        "ARGUSX_PROVIDER_BASE_URL",
        "ARGUSX_PROVIDER_API_KEY",
        "ARGUSX_PROVIDER_REPLAY_FILE",
    ] {
        std::env::remove_var(key);
    }
}
