use std::{path::PathBuf, sync::Arc};

use chrono::Utc;
use directories::ProjectDirs;
use reqwest::Url;
use uuid::Uuid;

use crate::provider_settings::{
    AesGcmSecretBox, EncryptedSecret, KeyringDataKeyStore, ProviderProfileStore,
    ProviderSettingsError,
    model::{
        ProviderConnectionResult, ProviderKind, ProviderProfileRecord, ProviderProfileSummary,
        ProviderRuntimeConfig, SaveProviderProfileInput, TestProviderProfileInput,
    },
};

#[derive(Clone)]
pub struct ProviderSettingsService {
    store: ProviderProfileStore,
    secret_box: AesGcmSecretBox,
}

impl ProviderSettingsService {
    pub fn new(store: ProviderProfileStore, secret_box: AesGcmSecretBox) -> Self {
        Self { store, secret_box }
    }

    pub fn from_default_location() -> Result<Self, ProviderSettingsError> {
        let store = ProviderProfileStore::new(default_db_path())?;
        let secret_box = AesGcmSecretBox::new(Arc::new(KeyringDataKeyStore::default()));
        Ok(Self::new(store, secret_box))
    }

    pub fn list_profiles(&self) -> Result<Vec<ProviderProfileSummary>, ProviderSettingsError> {
        self.store.list_profiles()
    }

    pub fn save_profile(
        &self,
        input: SaveProviderProfileInput,
    ) -> Result<ProviderProfileSummary, ProviderSettingsError> {
        let provider_kind = input.provider_kind;
        let name = trim_required("name", input.name)?;
        let base_url = normalize_url(trim_required("base_url", input.base_url)?)?;
        let model = trim_required("model", input.model)?;
        let existing = match input.id.as_deref() {
            Some(profile_id) => Some(
                self.store
                    .load_profile(profile_id)?
                    .ok_or_else(|| {
                        ProviderSettingsError::NotFound(format!(
                            "provider profile `{profile_id}` not found"
                        ))
                    })?,
            ),
            None => None,
        };
        self.ensure_provider_kind_constraints(provider_kind, existing.as_ref())?;

        let api_key = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let encrypted_secret = match (api_key, existing.as_ref()) {
            (Some(api_key), _) => self.secret_box.encrypt(&api_key)?,
            (None, Some(record)) => EncryptedSecret {
                ciphertext: record.api_key_ciphertext.clone(),
                nonce: record.api_key_nonce.clone(),
            },
            (None, None) => {
                return Err(ProviderSettingsError::Validation(
                    "api_key is required when creating a provider profile".to_string(),
                ));
            }
        };

        let had_default = self.store.has_default()?;
        let is_default = if !had_default {
            true
        } else if let Some(existing) = existing.as_ref() {
            if existing.is_default && !input.is_default {
                return Err(ProviderSettingsError::Validation(
                    "default profile cannot be unset without selecting another default"
                        .to_string(),
                ));
            }
            input.is_default
        } else {
            input.is_default
        };

        let now = Utc::now().to_rfc3339();
        let record = ProviderProfileRecord {
            id: existing
                .as_ref()
                .map(|record| record.id.clone())
                .or(input.id)
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            provider_kind,
            name,
            base_url,
            model,
            api_key_ciphertext: encrypted_secret.ciphertext,
            api_key_nonce: encrypted_secret.nonce,
            is_default,
            created_at: existing
                .as_ref()
                .map(|record| record.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
        };

        self.store.save_profile(&record)?;
        Ok(record.summary())
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<(), ProviderSettingsError> {
        let profile = self
            .store
            .load_profile(profile_id)?
            .ok_or_else(|| {
                ProviderSettingsError::NotFound(format!(
                    "provider profile `{profile_id}` not found"
                ))
            })?;

        if profile.is_default {
            return Err(ProviderSettingsError::Validation(
                "default provider profile cannot be deleted".to_string(),
            ));
        }

        self.store.delete_profile(profile_id)
    }

    pub fn set_default_profile(
        &self,
        profile_id: &str,
    ) -> Result<ProviderProfileSummary, ProviderSettingsError> {
        self.store
            .set_default_profile(profile_id)
            .map(|record| record.summary())
    }

    pub fn load_default_runtime_config(
        &self,
    ) -> Result<Option<ProviderRuntimeConfig>, ProviderSettingsError> {
        let Some(profile) = self.store.load_default_profile()? else {
            return Ok(None);
        };
        let api_key = self.secret_box.decrypt(&EncryptedSecret {
            ciphertext: profile.api_key_ciphertext,
            nonce: profile.api_key_nonce,
        })?;

        Ok(Some(ProviderRuntimeConfig {
            provider_kind: profile.provider_kind,
            base_url: profile.base_url,
            model: profile.model,
            api_key,
        }))
    }

    pub async fn test_profile(
        &self,
        input: TestProviderProfileInput,
    ) -> Result<ProviderConnectionResult, ProviderSettingsError> {
        let _provider_kind = input.provider_kind;
        let base_url = normalize_url(trim_required("base_url", input.base_url)?)?;
        let api_key = trim_required("api_key", input.api_key)?;
        let _model = trim_required("model", input.model)?;
        let endpoint = build_models_endpoint(&base_url)?;

        let response = reqwest::Client::new()
            .get(endpoint)
            .bearer_auth(api_key)
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(ProviderConnectionResult {
                success: true,
                message: "connection succeeded".to_string(),
            });
        }

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "empty response".to_string());

        Ok(ProviderConnectionResult {
            success: false,
            message: format!("{}: {}", status, body.trim()),
        })
    }

    fn ensure_provider_kind_constraints(
        &self,
        provider_kind: ProviderKind,
        existing: Option<&ProviderProfileRecord>,
    ) -> Result<(), ProviderSettingsError> {
        if !matches!(provider_kind, ProviderKind::Zai) {
            return Ok(());
        }

        let current_id = existing.map(|record| record.id.as_str());
        let has_other_zai = self
            .store
            .list_profiles()?
            .into_iter()
            .any(|profile| {
                profile.provider_kind == ProviderKind::Zai
                    && Some(profile.id.as_str()) != current_id
            });

        if has_other_zai {
            return Err(ProviderSettingsError::Validation(
                "Z.ai only supports a single saved profile".to_string(),
            ));
        }

        Ok(())
    }
}

fn default_db_path() -> PathBuf {
    ProjectDirs::from("com", "argusx", "argusx")
        .map(|dirs| dirs.data_local_dir().join("desktop.sqlite3"))
        .unwrap_or_else(|| std::env::temp_dir().join("argusx-desktop.sqlite3"))
}

fn build_models_endpoint(base_url: &str) -> Result<Url, ProviderSettingsError> {
    let mut url = Url::parse(base_url)
        .map_err(|err| ProviderSettingsError::Validation(err.to_string()))?;
    let mut path = url.path().trim_end_matches('/').to_string();
    if path.is_empty() {
        path.push('/');
    }
    if !path.ends_with("/models") {
        if !path.ends_with('/') {
            path.push('/');
        }
        path.push_str("models");
    }
    url.set_path(&path);
    Ok(url)
}

fn normalize_url(value: String) -> Result<String, ProviderSettingsError> {
    let parsed = Url::parse(value.trim())
        .map_err(|err| ProviderSettingsError::Validation(err.to_string()))?;
    Ok(parsed.to_string())
}

fn trim_required(field: &str, value: String) -> Result<String, ProviderSettingsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ProviderSettingsError::Validation(format!(
            "{field} is required"
        )));
    }
    Ok(trimmed.to_string())
}
