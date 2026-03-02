use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

const APP_SALT: &[u8] = b"argusx.desktop.runtime-config.v1";
const ENCRYPTION_CONTEXT: &[u8] = b"llm-runtime-config";
const NONCE_BYTES: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CipherEnvelope {
    pub v: u8,
    pub algo: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid cipher envelope version: {0}")]
    InvalidVersion(u8),
    #[error("unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
    #[error("failed to decode nonce: {0}")]
    NonceDecode(#[source] base64::DecodeError),
    #[error("failed to decode ciphertext: {0}")]
    CipherDecode(#[source] base64::DecodeError),
    #[error("invalid nonce length: expected {expected}, got {actual}")]
    InvalidNonceLength { expected: usize, actual: usize },
    #[error("cipher operation failed")]
    CipherOperation,
    #[error("decrypted payload is not valid UTF-8")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("failed to derive encryption key")]
    KeyDerivation,
}

pub fn derive_key_from_fingerprint(fingerprint: &str) -> Result<[u8; 32], CryptoError> {
    let hk = Hkdf::<Sha256>::new(Some(APP_SALT), fingerprint.trim().as_bytes());
    let mut key = [0_u8; 32];
    hk.expand(ENCRYPTION_CONTEXT, &mut key)
        .map_err(|_| CryptoError::KeyDerivation)?;
    Ok(key)
}

pub fn encrypt_secret(key: &[u8; 32], plaintext: &str) -> Result<CipherEnvelope, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::CipherOperation)?;
    let mut nonce = [0_u8; NONCE_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let nonce_ref = Nonce::from_slice(&nonce);

    let ciphertext = cipher
        .encrypt(nonce_ref, plaintext.as_bytes())
        .map_err(|_| CryptoError::CipherOperation)?;

    Ok(CipherEnvelope {
        v: 1,
        algo: "aes-256-gcm".to_string(),
        nonce_b64: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nonce),
        ciphertext_b64: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(ciphertext),
    })
}

pub fn decrypt_secret(key: &[u8; 32], envelope: &CipherEnvelope) -> Result<String, CryptoError> {
    if envelope.v != 1 {
        return Err(CryptoError::InvalidVersion(envelope.v));
    }
    if envelope.algo != "aes-256-gcm" {
        return Err(CryptoError::UnsupportedAlgorithm(envelope.algo.clone()));
    }

    let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&envelope.nonce_b64)
        .map_err(CryptoError::NonceDecode)?;
    if nonce.len() != NONCE_BYTES {
        return Err(CryptoError::InvalidNonceLength {
            expected: NONCE_BYTES,
            actual: nonce.len(),
        });
    }
    let ciphertext = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&envelope.ciphertext_b64)
        .map_err(CryptoError::CipherDecode)?;

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::CipherOperation)?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| CryptoError::CipherOperation)?;

    Ok(String::from_utf8(plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_is_stable_for_same_fingerprint() {
        let a = derive_key_from_fingerprint("fp-1").expect("derive key a");
        let b = derive_key_from_fingerprint("fp-1").expect("derive key b");
        assert_eq!(a, b);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = derive_key_from_fingerprint("fp").expect("derive key");
        let cipher = encrypt_secret(&key, "sk-test").expect("encrypt");
        let plain = decrypt_secret(&key, &cipher).expect("decrypt");
        assert_eq!(plain, "sk-test");
    }

    #[test]
    fn decrypt_fails_with_different_key() {
        let key_a = derive_key_from_fingerprint("fp-a").expect("derive key a");
        let key_b = derive_key_from_fingerprint("fp-b").expect("derive key b");
        let cipher = encrypt_secret(&key_a, "secret-value").expect("encrypt");
        let err = decrypt_secret(&key_b, &cipher).expect_err("must fail");
        assert!(matches!(err, CryptoError::CipherOperation));
    }
}
