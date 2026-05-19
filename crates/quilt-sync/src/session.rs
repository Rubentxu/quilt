//! E2E Encrypted Sync Session
//!
//! Establishes an encrypted sync session between two peers using X25519
//! key exchange and AES-256-GCM encryption.
//!
//! # Usage
//!
//! ```ignore
//! use quilt_sync::session::{SyncSession, SessionConfig};
//!
//! // Alice creates a session and gets her public key to share
//! let (alice_session, alice_public) = SyncSession::initiator();
//!
//! // Bob creates a session with Alice's public key
//! let bob_session = SyncSession::responder(alice_public).unwrap();
//! let bob_public = bob_session.public_key();
//!
//! // Alice completes the handshake with Bob's public key
//! alice_session.complete_handshake(bob_public).unwrap();
//!
//! // Now use encrypted_transport() to get an encrypted transport
//! let transport = bob_session.encrypted_transport(inner_transport);
//! ```

#[cfg(feature = "e2e-encryption")]
use crate::crypto::{CryptoManager, KeyExchangeManager, PublicKeyBytes};
#[cfg(feature = "e2e-encryption")]
use crate::transport::{EncryptedTransport, SyncTransport};

#[cfg(feature = "e2e-encryption")]
use thiserror::Error;

#[cfg(feature = "e2e-encryption")]
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Handshake not complete")]
    HandshakeIncomplete,

    #[error("Already completed handshake")]
    AlreadyCompleted,

    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),
}

#[cfg(feature = "e2e-encryption")]
pub struct SessionConfig {
    pub password: Option<String>,
}

#[cfg(feature = "e2e-encryption")]
impl Default for SessionConfig {
    fn default() -> Self {
        Self { password: None }
    }
}

#[cfg(feature = "e2e-encryption")]
pub struct SyncSession {
    key_manager: KeyExchangeManager,
    crypto_manager: Option<CryptoManager>,
    public_key: PublicKeyBytes,
    completed: bool,
}

#[cfg(feature = "e2e-encryption")]
impl SyncSession {
    pub fn initiator() -> (Self, PublicKeyBytes) {
        let (key_manager, public_key) = KeyExchangeManager::generate();
        let public_key_bytes = PublicKeyBytes::new(public_key);
        let session = Self {
            key_manager,
            crypto_manager: None,
            public_key: public_key_bytes.clone(),
            completed: false,
        };
        (session, public_key_bytes)
    }

    pub fn responder(remote_public: PublicKeyBytes) -> Result<Self, SessionError> {
        let (key_manager, public_key) = KeyExchangeManager::generate();
        let remote_pk = remote_public.to_public_key();
        let shared_secret = key_manager.derive_shared_secret(&remote_pk);
        let config = crate::crypto::CryptoConfig::from_shared_secret(shared_secret);
        let crypto_manager = CryptoManager::new(config)
            .map_err(|e| SessionError::KeyExchangeFailed(e.to_string()))?;

        Ok(Self {
            key_manager,
            crypto_manager: Some(crypto_manager),
            public_key: PublicKeyBytes::new(public_key),
            completed: true,
        })
    }

    pub fn public_key(&self) -> PublicKeyBytes {
        self.public_key.clone()
    }

    pub fn complete_handshake(
        mut self,
        remote_public: PublicKeyBytes,
    ) -> Result<Self, SessionError> {
        if self.completed {
            return Err(SessionError::AlreadyCompleted);
        }

        let remote_pk = remote_public.to_public_key();
        let shared_secret = self.key_manager.derive_shared_secret(&remote_pk);
        let config = crate::crypto::CryptoConfig::from_shared_secret(shared_secret);
        let crypto_manager = CryptoManager::new(config)
            .map_err(|e| SessionError::KeyExchangeFailed(e.to_string()))?;

        self.crypto_manager = Some(crypto_manager);
        self.completed = true;

        Ok(self)
    }

    pub fn is_complete(&self) -> bool {
        self.completed
    }

    pub fn encrypted_transport<T: SyncTransport + Send + Sync>(
        self,
        inner: T,
    ) -> Result<EncryptedTransport<T>, SessionError> {
        let crypto = self.crypto_manager.ok_or(SessionError::HandshakeIncomplete)?;
        Ok(EncryptedTransport::new(inner, crypto))
    }
}

#[cfg(test)]
#[cfg(feature = "e2e-encryption")]
mod tests {
    use super::*;

    #[test]
    fn test_initiator_and_responder() {
        let (alice_session, alice_public) = SyncSession::initiator();
        let bob_session = SyncSession::responder(alice_public.clone()).unwrap();

        assert!(bob_session.is_complete());
        assert_eq!(alice_session.public_key(), alice_public);
        assert_ne!(alice_session.public_key(), bob_session.public_key());
    }

    #[test]
    fn test_full_handshake() {
        let (mut alice_session, alice_public) = SyncSession::initiator();
        let bob_session = SyncSession::responder(alice_public).unwrap();
        let bob_public = bob_session.public_key();

        alice_session = alice_session.complete_handshake(bob_public).unwrap();

        assert!(alice_session.is_complete());
    }

    #[test]
    fn test_encrypted_transport() {
        let (alice_session, alice_public) = SyncSession::initiator();
        let bob_session = SyncSession::responder(alice_public).unwrap();
        let bob_public = bob_session.public_key();

        let alice_session = alice_session.complete_handshake(bob_public).unwrap();
        let _alice_encrypted = EncryptedTransport::new(
            crate::transport::MockTransport::default(),
            alice_session.crypto_manager.unwrap(),
        );
    }
}
