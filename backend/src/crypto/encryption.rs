use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use crate::crypto::error::CryptoError;
use base64::{Engine as _, engine::general_purpose};

const NONCE_SIZE: usize = 12; // 96 bits for GCM
const KEY_SIZE: usize = 32; // 256 bits

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptedData {
    pub ciphertext: String,
    pub nonce: String,
}

pub fn encrypt(plaintext: &[u8], key: &[u8]) -> Result<EncryptedData, CryptoError> {
    if key.len() != KEY_SIZE {
        return Err(CryptoError::InvalidKeyLength);
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::EncryptionFailed(format!("Failed to create cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(format!("Encryption failed: {}", e)))?;

    let engine = general_purpose::STANDARD;
    Ok(EncryptedData {
        ciphertext: engine.encode(&ciphertext),
        nonce: engine.encode(&nonce_bytes),
    })
}

pub fn decrypt(encrypted: &EncryptedData, key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key.len() != KEY_SIZE {
        return Err(CryptoError::InvalidKeyLength);
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::DecryptionFailed(format!("Failed to create cipher: {}", e)))?;

    let engine = base64::engine::general_purpose::STANDARD;
    let nonce_bytes = engine.decode(&encrypted.nonce)
        .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid nonce encoding: {}", e)))?;
    
    if nonce_bytes.len() != NONCE_SIZE {
        return Err(CryptoError::InvalidNonce);
    }

    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = engine.decode(&encrypted.ciphertext)
        .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid ciphertext encoding: {}", e)))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| CryptoError::DecryptionFailed(format!("Decryption failed: {}", e)))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_returns_original() {
        let plaintext = b"Hello, Black Vault!";
        let key = generate_test_key();
        
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn decryption_fails_with_wrong_key() {
        let plaintext = b"Hello, Black Vault!";
        let key1 = generate_test_key();
        let key2 = generate_different_test_key();
        
        let encrypted = encrypt(plaintext, &key1).unwrap();
        let result = decrypt(&encrypted, &key2);
        
        assert!(result.is_err());
    }

    #[test]
    fn decryption_fails_with_modified_ciphertext() {
        let plaintext = b"Hello, Black Vault!";
        let key = generate_test_key();
        
        let mut encrypted = encrypt(plaintext, &key).unwrap();
        encrypted.ciphertext.push_str("corrupted");
        
        let result = decrypt(&encrypted, &key);
        
        assert!(result.is_err());
    }

    #[test]
    fn decryption_fails_with_modified_nonce() {
        let plaintext = b"Hello, Black Vault!";
        let key = generate_test_key();
        
        let mut encrypted = encrypt(plaintext, &key).unwrap();
        encrypted.nonce = "invalidnonce".to_string();
        
        let result = decrypt(&encrypted, &key);
        
        assert!(result.is_err());
    }

    #[test]
    fn different_encryptions_use_fresh_nonces() {
        let plaintext = b"Hello, Black Vault!";
        let key = generate_test_key();
        
        let encrypted1 = encrypt(plaintext, &key).unwrap();
        let encrypted2 = encrypt(plaintext, &key).unwrap();
        
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        // Ciphertexts should also differ due to different nonces
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
    }

    #[test]
    fn encryption_fails_with_invalid_key_length() {
        let plaintext = b"Hello, Black Vault!";
        let short_key = vec![0u8; 16]; // Wrong size
        
        let result = encrypt(plaintext, &short_key);
        
        assert!(result.is_err());
    }

    #[test]
    fn decryption_fails_with_invalid_key_length() {
        let encrypted = EncryptedData {
            ciphertext: "test".to_string(),
            nonce: "test".to_string(),
        };
        let short_key = vec![0u8; 16]; // Wrong size
        
        let result = decrypt(&encrypted, &short_key);
        
        assert!(result.is_err());
    }

    fn generate_test_key() -> Vec<u8> {
        vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ]
    }

    fn generate_different_test_key() -> Vec<u8> {
        vec![
            0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
            0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30,
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40,
        ]
    }
}
