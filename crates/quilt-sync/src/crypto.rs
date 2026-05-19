//! E2E Encryption for sync data
//!
//! Provides AES-256-GCM encryption with Argon2id key derivation for
//! securing sync data in transit and at rest.
//!
//! # Security Model
//!
//! - **Encryption**: AES-256-GCM (authenticated encryption)
//! - **Key Derivation**: Argon2id (memory-hard, side-channel resistant)
//! - **Nonce**: Random 12-byte nonce per encryption (sent unencrypted)
//!
//! # Usage
//!
//! ```ignore
//! use quilt_sync::crypto::{CryptoManager, CryptoConfig};
//!
//! let config = CryptoConfig::from_password("my-secret-password");
//! let crypto = CryptoManager::new(config).unwrap();
//!
//! // Encrypt a sync change
//! let encrypted = crypto.encrypt_change(change).unwrap();
//!
//! // Decrypt a sync change
//! let decrypted = crypto.decrypt_change(encrypted).unwrap();
//! ```

use crate::crdt::SyncChange;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "e2e-encryption")]
use x25519_dalek::{PublicKey, StaticSecret};

#[cfg(feature = "e2e-encryption")]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
#[cfg(feature = "e2e-encryption")]
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
#[cfg(feature = "e2e-encryption")]
use rand::{rngs::OsRng, RngCore};

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Invalid ciphertext: data too short")]
    InvalidCiphertext,

    #[error("Ciphertext authentication failed (tampered data)")]
    AuthenticationFailed,
}

#[cfg(feature = "e2e-encryption")]
impl From<aes_gcm::Error> for CryptoError {
    fn from(e: aes_gcm::Error) -> Self {
        CryptoError::EncryptionFailed(e.to_string())
    }
}

#[cfg(feature = "e2e-encryption")]
impl From<argon2::Error> for CryptoError {
    fn from(e: argon2::Error) -> Self {
        CryptoError::KeyDerivationFailed(e.to_string())
    }
}

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;
const SALT_SIZE: usize = 16;

#[cfg(feature = "e2e-encryption")]
#[derive(Clone)]
pub struct CryptoManager {
    cipher: Aes256Gcm,
}

#[cfg(feature = "e2e-encryption")]
impl CryptoManager {
    pub fn new(config: CryptoConfig) -> Result<Self, CryptoError> {
        let key = config.derive_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        Ok(Self { cipher })
    }

    pub fn encrypt_data(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed("Encryption failed".to_string()))?;

        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if data.len() < NONCE_SIZE {
            return Err(CryptoError::InvalidCiphertext);
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::AuthenticationFailed)
    }

    pub fn encrypt_change(&self, mut change: SyncChange) -> Result<SyncChange, CryptoError> {
        let encrypted_data = self.encrypt_data(&change.data)?;
        change.data = encrypted_data;
        Ok(change)
    }

    pub fn decrypt_change(&self, mut change: SyncChange) -> Result<SyncChange, CryptoError> {
        let decrypted_data = self.decrypt_data(&change.data)?;
        change.data = decrypted_data;
        Ok(change)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    password: Option<String>,
    salt: Option<Vec<u8>>,
    raw_key: Option<[u8; KEY_SIZE]>,
}

impl CryptoConfig {
    pub fn from_password(password: impl Into<String>) -> Self {
        Self {
            password: Some(password.into()),
            salt: None,
            raw_key: None,
        }
    }

    pub fn from_password_with_salt(password: impl Into<String>, salt: Vec<u8>) -> Self {
        Self {
            password: Some(password.into()),
            salt: Some(salt),
            raw_key: None,
        }
    }

    #[cfg(feature = "e2e-encryption")]
    pub fn from_shared_secret(shared_secret: [u8; 32]) -> Self {
        Self {
            password: None,
            salt: None,
            raw_key: Some(shared_secret),
        }
    }

    #[cfg(feature = "e2e-encryption")]
    fn derive_key(&self) -> Result<[u8; KEY_SIZE], CryptoError> {
        if let Some(raw_key) = &self.raw_key {
            return Ok(*raw_key);
        }

        let password = self.password.as_ref()
            .ok_or_else(|| CryptoError::KeyDerivationFailed("No password or key provided".to_string()))?;

        let salt = match &self.salt {
            Some(s) if s.len() == SALT_SIZE => s.clone(),
            _ => {
                let mut salt_bytes = vec![0u8; SALT_SIZE];
                OsRng.fill_bytes(&mut salt_bytes);
                salt_bytes
            }
        };

        let salt_string = SaltString::encode_b64(&salt)
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

        let hash_output = hash.hash.ok_or_else(|| {
            CryptoError::KeyDerivationFailed("No hash output".to_string())
        })?;

        let mut key = [0u8; KEY_SIZE];
        let hash_bytes = hash_output.as_bytes();
        let len = std::cmp::min(hash_bytes.len(), KEY_SIZE);
        key[..len].copy_from_slice(&hash_bytes[..len]);

        Ok(key)
    }

    pub fn get_salt(&self) -> Option<Vec<u8>> {
        self.salt.clone()
    }
}

#[cfg(feature = "e2e-encryption")]
impl CryptoManager {
    pub fn encrypt_change_with_new_nonce(
        &self,
        mut change: SyncChange,
    ) -> Result<(SyncChange, Vec<u8>), CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);

        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, change.data.as_slice())
            .map_err(|_| CryptoError::EncryptionFailed("Encryption failed".to_string()))?;

        let mut encrypted_data = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        encrypted_data.extend_from_slice(&nonce_bytes);
        encrypted_data.extend_from_slice(&ciphertext);

        change.data = encrypted_data;
        Ok((change, nonce_bytes.to_vec()))
    }
}

#[cfg(feature = "e2e-encryption")]
#[derive(Clone)]
pub struct KeyExchangeManager {
    static_secret: StaticSecret,
}

#[cfg(feature = "e2e-encryption")]
impl KeyExchangeManager {
    pub fn generate() -> (Self, PublicKey) {
        let static_secret = StaticSecret::random_from_rng(OsRng);
        let public_key = PublicKey::from(&static_secret);
        (Self { static_secret }, public_key)
    }

    pub fn derive_shared_secret(&self, remote_public_key: &PublicKey) -> [u8; 32] {
        let shared_secret = self.static_secret.diffie_hellman(remote_public_key);
        *shared_secret.as_bytes()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::from(&self.static_secret)
    }

    pub fn into_crypto_manager(self) -> Result<CryptoManager, CryptoError> {
        let shared_secret = self.static_secret.diffie_hellman(&self.public_key());
        let config = CryptoConfig::from_shared_secret(*shared_secret.as_bytes());
        CryptoManager::new(config)
    }
}

#[cfg(feature = "e2e-encryption")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyBytes([u8; 32]);

#[cfg(feature = "e2e-encryption")]
impl PublicKeyBytes {
    pub fn new(key: PublicKey) -> Self {
        Self(*key.as_bytes())
    }

    pub fn to_public_key(&self) -> PublicKey {
        PublicKey::from(self.0)
    }
}

#[cfg(test)]
#[cfg(feature = "e2e-encryption")]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let config = CryptoConfig::from_password("test-password-123");
        let crypto = CryptoManager::new(config).unwrap();

        let plaintext = b"Hello, World! This is secret data.";
        let encrypted = crypto.encrypt_data(plaintext).unwrap();
        let decrypted = crypto.decrypt_data(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_different_nonces_each_time() {
        let config = CryptoConfig::from_password("test-password");
        let crypto = CryptoManager::new(config).unwrap();

        let plaintext = b"Same data";
        let enc1 = crypto.encrypt_data(plaintext).unwrap();
        let enc2 = crypto.encrypt_data(plaintext).unwrap();

        assert_ne!(enc1, enc2, "Same plaintext should produce different ciphertexts");
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let config = CryptoConfig::from_password("test-password");
        let crypto = CryptoManager::new(config).unwrap();

        let plaintext = b"Secret message";
        let mut encrypted = crypto.encrypt_data(plaintext).unwrap();

        encrypted.push(0xFF);

        let result = crypto.decrypt_data(&encrypted);
        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));
    }

    #[test]
    fn test_change_encrypt_decrypt() {
        let config = CryptoConfig::from_password("secret");
        let crypto = CryptoManager::new(config).unwrap();

        let change = SyncChange {
            entity_id: uuid::Uuid::new_v4(),
            entity_type: "block".to_string(),
            data: b"Block content here".to_vec(),
            version: 1,
            peer_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        let encrypted = crypto.encrypt_change(change.clone()).unwrap();
        let decrypted = crypto.decrypt_change(encrypted).unwrap();

        assert_eq!(change.data, decrypted.data);
        assert_eq!(change.entity_id, decrypted.entity_id);
        assert_eq!(change.entity_type, decrypted.entity_type);
    }

    #[test]
    fn test_wrong_password_fails() {
        let config1 = CryptoConfig::from_password("password1");
        let config2 = CryptoConfig::from_password("password2");

        let crypto1 = CryptoManager::new(config1).unwrap();
        let crypto2 = CryptoManager::new(config2).unwrap();

        let plaintext = b"Secret data";
        let encrypted = crypto1.encrypt_data(plaintext).unwrap();

        let result = crypto2.decrypt_data(&encrypted);
        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));
    }

    #[test]
    fn test_key_exchange_derive_shared_secret() {
        let (alice_manager, alice_public) = KeyExchangeManager::generate();
        let (bob_manager, bob_public) = KeyExchangeManager::generate();

        let alice_shared = alice_manager.derive_shared_secret(&bob_public);
        let bob_shared = bob_manager.derive_shared_secret(&alice_public);

        assert_eq!(alice_shared, bob_shared, "Shared secrets should match");
    }

    #[test]
    fn test_key_exchange_encrypt_decrypt() {
        let (alice_manager, alice_public) = KeyExchangeManager::generate();
        let (bob_manager, bob_public) = KeyExchangeManager::generate();

        let alice_shared = alice_manager.derive_shared_secret(&bob_public);
        let alice_config = CryptoConfig::from_shared_secret(alice_shared);
        let alice_crypto = CryptoManager::new(alice_config).unwrap();

        let bob_shared = bob_manager.derive_shared_secret(&alice_public);
        let bob_config = CryptoConfig::from_shared_secret(bob_shared);
        let bob_crypto = CryptoManager::new(bob_config).unwrap();

        let plaintext = b"Key exchange test message";
        let encrypted = alice_crypto.encrypt_data(plaintext).unwrap();
        let decrypted = bob_crypto.decrypt_data(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }
}
