pub mod commands;
pub mod crypto;
pub mod model;
pub mod service;
pub mod store;

pub use commands::{
    delete_provider_profile, list_provider_profiles, save_provider_profile,
    set_default_provider_profile, test_provider_profile,
};
pub use crypto::{AesGcmSecretBox, DataKeyStore, EncryptedSecret, KeyringDataKeyStore};
pub use model::{
    ProviderConnectionResult, ProviderKind, ProviderProfileSummary, ProviderRuntimeConfig,
    SaveProviderProfileInput, TestProviderProfileInput,
};
pub use service::ProviderSettingsService;
pub use store::ProviderProfileStore;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderSettingsError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    SecureStorage(String),
    #[error("{0}")]
    Crypto(String),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}
