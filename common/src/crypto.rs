use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed / invalid auth tag: {0}")]
    DecryptionFailed(String),
    #[error("Invalid base64 payload: {0}")]
    InvalidBase64(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Entropy error: {0}")]
    EntropyError(String),
}

/// Ephemeral end-to-end encrypted envelope suitable for relaying over untrusted networks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub topic: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
    pub created_at: i64,
}

/// Derive a 256-bit symmetric AES key deterministically from a passphrase and salt.
pub fn derive_key(passphrase: &str, salt: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"EATTURA_E2EE_KEY_SALT_v1:");
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(passphrase.as_bytes());
    hasher.finalize().into()
}

/// Derive a deterministic, opaque topic hash from a passphrase for relay mailbox routing.
/// The relay server sees only this opaque hash and never the plaintext passphrase.
pub fn derive_topic(passphrase: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"EATTURA_RELAY_TOPIC_SALT_v1:");
    hasher.update(passphrase.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Encrypt arbitrary plaintext bytes using AES-256-GCM.
pub fn encrypt_bytes(
    plaintext: &[u8],
    key_bytes: &[u8; 32],
    topic: &str,
) -> Result<EncryptedEnvelope, CryptoError> {
    let key = Key::<Aes256Gcm>::from(*key_bytes);
    let cipher = Aes256Gcm::new(&key);

    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).map_err(|e| CryptoError::EntropyError(e.to_string()))?;
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    Ok(EncryptedEnvelope {
        topic: topic.to_string(),
        nonce_b64: BASE64_STANDARD.encode(nonce_bytes),
        ciphertext_b64: BASE64_STANDARD.encode(&ciphertext),
        created_at: now,
    })
}

/// Decrypt an encrypted envelope using AES-256-GCM.
pub fn decrypt_envelope(
    envelope: &EncryptedEnvelope,
    key_bytes: &[u8; 32],
) -> Result<Vec<u8>, CryptoError> {
    let key = Key::<Aes256Gcm>::from(*key_bytes);
    let cipher = Aes256Gcm::new(&key);

    let nonce_raw = BASE64_STANDARD
        .decode(&envelope.nonce_b64)
        .map_err(|e| CryptoError::InvalidBase64(e.to_string()))?;

    if nonce_raw.len() != 12 {
        return Err(CryptoError::DecryptionFailed(
            "Invalid nonce length, expected 12 bytes".to_string(),
        ));
    }

    let nonce = Nonce::try_from(nonce_raw.as_slice()).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    let ciphertext = BASE64_STANDARD
        .decode(&envelope.ciphertext_b64)
        .map_err(|e| CryptoError::InvalidBase64(e.to_string()))?;

    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2ee_encryption_roundtrip() {
        let passphrase = "spada-aquila-foresta-vittoria-2026";
        let key = derive_key(passphrase, "test-salt");
        let topic = derive_topic(passphrase);

        let secret_msg = b"Fattura 2026/001 con dati fiscali riservati";
        let envelope = encrypt_bytes(secret_msg, &key, &topic).unwrap();

        assert_eq!(envelope.topic, topic);
        assert!(!envelope.ciphertext_b64.is_empty());

        let decrypted = decrypt_envelope(&envelope, &key).unwrap();
        assert_eq!(decrypted, secret_msg);

        // Decryption with wrong key must fail
        let wrong_key = derive_key("chiave-errata", "test-salt");
        assert!(decrypt_envelope(&envelope, &wrong_key).is_err());
    }
}
