use serde::{Deserialize, Serialize};
use crate::crypto::{self, CryptoError, EncryptedEnvelope};

/// Standard multi-device sovereign synchronization payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPacket {
    pub version: u32,
    pub sender_device_id: String,
    pub timestamp: i64,
    pub invoices: Vec<SyncInvoiceItem>,
    pub clients: Vec<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
}

/// Individual invoice item bundled with its canonical XML content and associated SDI notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInvoiceItem {
    pub id: String,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub direction: String,
    pub xml_content: String,
    pub sdi_notifications: Vec<(String, String)>, // (tipo, raw_xml)
}

impl SyncPacket {
    pub fn new(sender_device_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Self {
            version: 1,
            sender_device_id,
            timestamp: now,
            invoices: Vec::new(),
            clients: Vec::new(),
            settings: None,
        }
    }

    /// Encrypt this sync packet with a user-supplied passphrase into an opaque relay envelope.
    pub fn encrypt(&self, passphrase: &str, salt: &str) -> Result<EncryptedEnvelope, CryptoError> {
        let key = crypto::derive_key(passphrase, salt);
        let topic = crypto::derive_topic(passphrase);
        let json_bytes = serde_json::to_vec(self)
            .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

        crypto::encrypt_bytes(&json_bytes, &key, &topic)
    }

    /// Decrypt an opaque relay envelope into a typed sync packet.
    pub fn decrypt(envelope: &EncryptedEnvelope, passphrase: &str, salt: &str) -> Result<Self, CryptoError> {
        let key = crypto::derive_key(passphrase, salt);
        let bytes = crypto::decrypt_envelope(envelope, &key)?;

        serde_json::from_slice(&bytes)
            .map_err(|e| CryptoError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_packet_encrypt_decrypt_roundtrip() {
        let mut packet = SyncPacket::new("macbook-air-m2".to_string());
        packet.invoices.push(SyncInvoiceItem {
            id: "inv-001".to_string(),
            numero: "2026/01".to_string(),
            data: "2026-03-01".to_string(),
            tipo_documento: "TD01".to_string(),
            direction: "sale".to_string(),
            xml_content: "<FatturaElettronica>...</FatturaElettronica>".to_string(),
            sdi_notifications: vec![("RC".to_string(), "<RicevutaConsegna/>".to_string())],
        });
        packet.clients.push(serde_json::json!({
            "id": "cli-123",
            "denominazione": "Studio Rossi",
            "id_codice": "12345678901"
        }));

        let passphrase = "luna-stella-fiume-tempo-2026";
        let envelope = packet.encrypt(passphrase, "org-vault-sync").unwrap();

        // Topic is derived and opaque
        assert_eq!(envelope.topic, crypto::derive_topic(passphrase));

        // Decrypt
        let recovered = SyncPacket::decrypt(&envelope, passphrase, "org-vault-sync").unwrap();
        assert_eq!(recovered.sender_device_id, "macbook-air-m2");
        assert_eq!(recovered.invoices.len(), 1);
        assert_eq!(recovered.invoices[0].numero, "2026/01");
        assert_eq!(recovered.clients.len(), 1);

        // Wrong passphrase fails
        assert!(SyncPacket::decrypt(&envelope, "wrong-passphrase", "org-vault-sync").is_err());
    }
}
