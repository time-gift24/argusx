use std::{path::PathBuf, sync::Arc};

use desktop_lib::provider_settings::{
    AesGcmSecretBox, DataKeyStore, ProviderKind, ProviderProfileStore,
    ProviderSettingsError, ProviderSettingsService, SaveProviderProfileInput,
};
use rusqlite::Connection;
use uuid::Uuid;

#[test]
fn save_profile_lists_summaries_and_reassigns_default() {
    let db_path = temp_db_path("provider-settings-defaults");
    let service = test_service(&db_path);

    let first = service
        .save_profile(SaveProviderProfileInput {
            id: None,
            name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1/".into(),
            model: "openai/gpt-4.1-mini".into(),
            api_key: Some("sk-openrouter".into()),
            is_default: true,
        })
        .unwrap();

    let second = service
        .save_profile(SaveProviderProfileInput {
            id: None,
            name: "Local vLLM".into(),
            base_url: "http://127.0.0.1:8000/v1/".into(),
            model: "deepseek-v3".into(),
            api_key: Some("sk-local".into()),
            is_default: true,
        })
        .unwrap();

    let profiles = service.list_profiles().unwrap();
    let default_ids = profiles
        .iter()
        .filter(|profile| profile.is_default)
        .map(|profile| profile.id.clone())
        .collect::<Vec<_>>();

    assert_eq!(profiles.len(), 2);
    assert_eq!(default_ids, vec![second.id.clone()]);
    assert_eq!(profiles[0].provider_kind, ProviderKind::OpenAiCompatible);
    assert_eq!(first.provider_kind, ProviderKind::OpenAiCompatible);
}

#[test]
fn save_profile_encrypts_api_key_and_keeps_existing_secret_on_blank_update() {
    let db_path = temp_db_path("provider-settings-secrets");
    let service = test_service(&db_path);

    let profile = service
        .save_profile(SaveProviderProfileInput {
            id: None,
            name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1/".into(),
            model: "openai/gpt-4.1-mini".into(),
            api_key: Some("sk-original".into()),
            is_default: true,
        })
        .unwrap();

    let ciphertext = raw_ciphertext(&db_path, &profile.id);
    assert!(!ciphertext.is_empty());
    assert_ne!(ciphertext, b"sk-original");

    let updated = service
        .save_profile(SaveProviderProfileInput {
            id: Some(profile.id.clone()),
            name: "OpenRouter Stable".into(),
            base_url: "https://openrouter.ai/api/v1/".into(),
            model: "openai/gpt-4.1".into(),
            api_key: None,
            is_default: true,
        })
        .unwrap();

    let runtime = service.load_default_runtime_config().unwrap().unwrap();

    assert_eq!(updated.id, profile.id);
    assert_eq!(updated.name, "OpenRouter Stable");
    assert_eq!(runtime.api_key, "sk-original");
    assert_eq!(runtime.model, "openai/gpt-4.1");
}

#[test]
fn delete_default_profile_is_rejected() {
    let db_path = temp_db_path("provider-settings-delete");
    let service = test_service(&db_path);

    let profile = service
        .save_profile(SaveProviderProfileInput {
            id: None,
            name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1/".into(),
            model: "openai/gpt-4.1-mini".into(),
            api_key: Some("sk-openrouter".into()),
            is_default: true,
        })
        .unwrap();

    let error = service.delete_profile(&profile.id).unwrap_err();

    assert!(matches!(
        error,
        ProviderSettingsError::Validation(message) if message.contains("default")
    ));
}

struct FixedKeyStore;

impl DataKeyStore for FixedKeyStore {
    fn load_or_create_key(&self) -> Result<[u8; 32], ProviderSettingsError> {
        Ok([7; 32])
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

fn raw_ciphertext(db_path: &PathBuf, profile_id: &str) -> Vec<u8> {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row(
        "SELECT api_key_ciphertext FROM provider_profiles WHERE id = ?1",
        [profile_id],
        |row| row.get::<_, Vec<u8>>(0),
    )
    .unwrap()
}
