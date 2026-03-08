use std::sync::Arc;

use aead::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use keyring::{Entry, Error as KeyringError};

use crate::provider_settings::ProviderSettingsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedSecret {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub trait DataKeyStore: Send + Sync {
    fn load_or_create_key(&self) -> Result<[u8; 32], ProviderSettingsError>;
}

#[derive(Clone)]
pub struct AesGcmSecretBox {
    data_key_store: Arc<dyn DataKeyStore>,
}

impl AesGcmSecretBox {
    pub fn new(data_key_store: Arc<dyn DataKeyStore>) -> Self {
        Self { data_key_store }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedSecret, ProviderSettingsError> {
        let key = self.data_key_store.load_or_create_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|err| ProviderSettingsError::Crypto(err.to_string()))?;
        let mut nonce = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
            .map_err(|err| ProviderSettingsError::Crypto(err.to_string()))?;

        Ok(EncryptedSecret {
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    pub fn decrypt(&self, encrypted: &EncryptedSecret) -> Result<String, ProviderSettingsError> {
        let key = self.data_key_store.load_or_create_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|err| ProviderSettingsError::Crypto(err.to_string()))?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&encrypted.nonce), encrypted.ciphertext.as_ref())
            .map_err(|err| ProviderSettingsError::Crypto(err.to_string()))?;

        String::from_utf8(plaintext).map_err(|err| ProviderSettingsError::Crypto(err.to_string()))
    }
}

pub struct KeyringDataKeyStore {
    service: String,
    user: String,
}

impl Default for KeyringDataKeyStore {
    fn default() -> Self {
        Self {
            service: "argusx.desktop.provider_settings".to_string(),
            user: "data-encryption-key".to_string(),
        }
    }
}

impl KeyringDataKeyStore {
    fn entry(&self) -> Result<Entry, ProviderSettingsError> {
        Entry::new(&self.service, &self.user)
            .map_err(|err| ProviderSettingsError::SecureStorage(err.to_string()))
    }
}

impl DataKeyStore for KeyringDataKeyStore {
    fn load_or_create_key(&self) -> Result<[u8; 32], ProviderSettingsError> {
        let entry = self.entry()?;

        match entry.get_secret() {
            Ok(secret) => slice_to_key(&secret),
            Err(KeyringError::NoEntry) => {
                let mut key = [0_u8; 32];
                OsRng.fill_bytes(&mut key);
                entry
                    .set_secret(&key)
                    .map_err(|err| ProviderSettingsError::SecureStorage(err.to_string()))?;
                Ok(key)
            }
            Err(err) => Err(ProviderSettingsError::SecureStorage(err.to_string())),
        }
    }
}

fn slice_to_key(value: &[u8]) -> Result<[u8; 32], ProviderSettingsError> {
    if value.len() != 32 {
        return Err(ProviderSettingsError::SecureStorage(
            "stored encryption key has invalid length".to_string(),
        ));
    }

    let mut key = [0_u8; 32];
    key.copy_from_slice(value);
    Ok(key)
}
