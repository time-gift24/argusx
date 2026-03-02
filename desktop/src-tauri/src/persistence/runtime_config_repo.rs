use std::path::PathBuf;

use rusqlite::{params, OptionalExtension};
use thiserror::Error;

use crate::llm_runtime_config::{
    normalize_runtime_config, HeaderPair, LlmRuntimeConfig, ProviderId, ProviderRuntimeConfig,
};
use crate::secure_config::{
    decrypt_secret, derive_key_from_fingerprint, encrypt_secret, load_host_fingerprint,
    CipherEnvelope, CryptoError, HostFingerprintError,
};

use super::{open_and_bootstrap, SchemaError};

#[derive(Debug, Error)]
pub enum RuntimeConfigRepoError {
    #[error("schema error: {0}")]
    Schema(#[from] SchemaError),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("failed to load host fingerprint: {0}")]
    HostFingerprint(#[from] HostFingerprintError),
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("stored config bound to different machine fingerprint")]
    FingerprintMismatch,
    #[error("invalid provider id in storage: {0}")]
    InvalidProviderId(String),
}

pub struct RuntimeConfigRepo {
    db_path: PathBuf,
}

impl RuntimeConfigRepo {
    pub fn new(db_path: PathBuf) -> Result<Self, RuntimeConfigRepoError> {
        let _ = open_and_bootstrap(&db_path)?;
        Ok(Self { db_path })
    }

    fn open_connection(&self) -> Result<rusqlite::Connection, RuntimeConfigRepoError> {
        open_and_bootstrap(&self.db_path).map_err(Into::into)
    }

    pub fn save(
        &self,
        config: &LlmRuntimeConfig,
    ) -> Result<LlmRuntimeConfig, RuntimeConfigRepoError> {
        let fingerprint = load_host_fingerprint()?;
        self.save_with_fingerprint(config, &fingerprint)
    }

    pub fn load(&self) -> Result<Option<LlmRuntimeConfig>, RuntimeConfigRepoError> {
        let fingerprint = load_host_fingerprint()?;
        self.load_with_fingerprint(&fingerprint)
    }

    pub fn clear(&self) -> Result<(), RuntimeConfigRepoError> {
        let conn = self.open_connection()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM llm_provider_configs", [])?;
        tx.execute("DELETE FROM llm_runtime_config", [])?;
        tx.commit()?;
        Ok(())
    }

    pub fn save_with_fingerprint(
        &self,
        config: &LlmRuntimeConfig,
        fingerprint: &str,
    ) -> Result<LlmRuntimeConfig, RuntimeConfigRepoError> {
        let normalized = normalize_runtime_config(config.clone());
        let encryption_key = derive_key_from_fingerprint(fingerprint)?;
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.open_connection()?;
        let tx = conn.unchecked_transaction()?;

        tx.execute(
            r#"
INSERT INTO llm_runtime_config (id, default_provider, updated_at_ms)
VALUES (1, ?1, ?2)
ON CONFLICT(id) DO UPDATE SET
  default_provider = excluded.default_provider,
  updated_at_ms = excluded.updated_at_ms
"#,
            params![
                normalized
                    .default_provider
                    .as_ref()
                    .map(ProviderId::as_adapter_id),
                now
            ],
        )?;

        for provider in [
            ProviderId::Bigmodel,
            ProviderId::Openai,
            ProviderId::Anthropic,
        ] {
            let provider_cfg = normalized.provider(&provider);
            let models_json = serde_json::to_string(&provider_cfg.models)?;
            let headers_json = serde_json::to_string(&provider_cfg.headers)?;
            let api_key_cipher_json = if provider_cfg.api_key.trim().is_empty() {
                String::new()
            } else {
                let cipher = encrypt_secret(&encryption_key, &provider_cfg.api_key)?;
                serde_json::to_string(&cipher)?
            };

            tx.execute(
                r#"
INSERT INTO llm_provider_configs (
  provider_id, base_url, models_json, headers_json, api_key_cipher_json, updated_at_ms
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6)
ON CONFLICT(provider_id) DO UPDATE SET
  base_url = excluded.base_url,
  models_json = excluded.models_json,
  headers_json = excluded.headers_json,
  api_key_cipher_json = excluded.api_key_cipher_json,
  updated_at_ms = excluded.updated_at_ms
"#,
                params![
                    provider.as_adapter_id(),
                    provider_cfg.base_url,
                    models_json,
                    headers_json,
                    api_key_cipher_json,
                    now
                ],
            )?;
        }

        tx.commit()?;
        Ok(normalized)
    }

    pub fn load_with_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<LlmRuntimeConfig>, RuntimeConfigRepoError> {
        let mut config = LlmRuntimeConfig::default();
        let encryption_key = derive_key_from_fingerprint(fingerprint)?;
        let conn = self.open_connection()?;

        let default_provider: Option<String> = conn
            .query_row(
                "SELECT default_provider FROM llm_runtime_config WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        config.default_provider = default_provider
            .as_deref()
            .map(provider_from_id)
            .transpose()?;

        let mut found_provider_row = false;
        let mut stmt = conn.prepare(
            r#"
SELECT provider_id, base_url, models_json, headers_json, api_key_cipher_json
FROM llm_provider_configs
"#,
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            found_provider_row = true;
            let provider_id: String = row.get(0)?;
            let provider = provider_from_id(&provider_id)?;
            let base_url: String = row.get(1)?;
            let models_json: String = row.get(2)?;
            let headers_json: String = row.get(3)?;
            let api_key_cipher_json: String = row.get(4)?;

            let models: Vec<String> = serde_json::from_str(&models_json)?;
            let headers: Vec<HeaderPair> = serde_json::from_str(&headers_json)?;
            let api_key = if api_key_cipher_json.trim().is_empty() {
                String::new()
            } else {
                let envelope: CipherEnvelope = serde_json::from_str(&api_key_cipher_json)?;
                decrypt_secret(&encryption_key, &envelope).map_err(map_decrypt_error)?
            };

            set_provider_config(
                &mut config,
                &provider,
                ProviderRuntimeConfig {
                    api_key,
                    base_url,
                    models,
                    headers,
                },
            );
        }

        if !found_provider_row && config.default_provider.is_none() {
            return Ok(None);
        }

        Ok(Some(normalize_runtime_config(config)))
    }
}

fn map_decrypt_error(err: CryptoError) -> RuntimeConfigRepoError {
    match err {
        CryptoError::CipherOperation => RuntimeConfigRepoError::FingerprintMismatch,
        other => RuntimeConfigRepoError::Crypto(other),
    }
}

fn provider_from_id(value: &str) -> Result<ProviderId, RuntimeConfigRepoError> {
    match value {
        "bigmodel" => Ok(ProviderId::Bigmodel),
        "openai" => Ok(ProviderId::Openai),
        "anthropic" => Ok(ProviderId::Anthropic),
        _ => Err(RuntimeConfigRepoError::InvalidProviderId(value.to_string())),
    }
}

fn set_provider_config(
    config: &mut LlmRuntimeConfig,
    provider: &ProviderId,
    provider_config: ProviderRuntimeConfig,
) {
    match provider {
        ProviderId::Bigmodel => config.providers.bigmodel = provider_config,
        ProviderId::Openai => config.providers.openai = provider_config,
        ProviderId::Anthropic => config.providers.anthropic = provider_config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_runtime_config() -> LlmRuntimeConfig {
        LlmRuntimeConfig {
            default_provider: Some(ProviderId::Openai),
            providers: crate::llm_runtime_config::ProviderConfigs {
                openai: ProviderRuntimeConfig {
                    api_key: "sk-openai-test".to_string(),
                    base_url: "https://openai.provider.test/v1".to_string(),
                    models: vec!["gpt-4o".to_string()],
                    headers: vec![HeaderPair {
                        key: "X-Test".to_string(),
                        value: "1".to_string(),
                    }],
                },
                ..crate::llm_runtime_config::ProviderConfigs::default()
            },
        }
    }

    #[test]
    fn runtime_config_repo_encrypts_api_key_at_rest() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("runtime-config.db");
        let repo = RuntimeConfigRepo::new(db_path.clone()).expect("create repo");
        let config = sample_runtime_config();

        repo.save_with_fingerprint(&config, "fp-a")
            .expect("save config");

        let conn = open_and_bootstrap(&db_path).expect("open sqlite");
        let cipher_json: String = conn
            .query_row(
                "SELECT api_key_cipher_json FROM llm_provider_configs WHERE provider_id = 'openai'",
                [],
                |row| row.get(0),
            )
            .expect("query cipher");

        assert!(!cipher_json.is_empty());
        assert!(!cipher_json.contains("sk-openai-test"));
    }

    #[test]
    fn runtime_config_repo_roundtrips_with_same_fingerprint() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("runtime-config.db");
        let repo = RuntimeConfigRepo::new(db_path).expect("create repo");
        let config = sample_runtime_config();

        repo.save_with_fingerprint(&config, "fp-a")
            .expect("save config");
        let loaded = repo
            .load_with_fingerprint("fp-a")
            .expect("load config")
            .expect("config exists");

        assert_eq!(loaded.default_provider, Some(ProviderId::Openai));
        assert_eq!(loaded.providers.openai.api_key, "sk-openai-test");
        assert_eq!(
            loaded.providers.openai.base_url,
            "https://openai.provider.test/v1"
        );
        assert_eq!(loaded.providers.openai.models, vec!["gpt-4o"]);
    }

    #[test]
    fn runtime_config_repo_rejects_mismatched_fingerprint() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("runtime-config.db");
        let repo = RuntimeConfigRepo::new(db_path).expect("create repo");
        let config = sample_runtime_config();

        repo.save_with_fingerprint(&config, "fp-a")
            .expect("save config");
        let err = repo
            .load_with_fingerprint("fp-b")
            .expect_err("must reject mismatched fingerprint");
        assert!(matches!(err, RuntimeConfigRepoError::FingerprintMismatch));
    }
}
