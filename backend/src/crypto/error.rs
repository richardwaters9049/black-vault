#[derive(Debug)]
pub enum CryptoError {
    EncryptionFailed(String),
    DecryptionFailed(String),
    KeyDerivationFailed(String),
    InvalidKeyLength,
    InvalidNonce,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
            CryptoError::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
            CryptoError::KeyDerivationFailed(msg) => write!(f, "Key derivation failed: {}", msg),
            CryptoError::InvalidKeyLength => write!(f, "Invalid key length"),
            CryptoError::InvalidNonce => write!(f, "Invalid nonce"),
        }
    }
}

impl std::error::Error for CryptoError {}
