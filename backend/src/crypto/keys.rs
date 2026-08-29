use crate::crypto::error::CryptoError;
use rand::RngCore;

const VAULT_KEY_SIZE: usize = 32; // 256 bits
const FILE_KEY_SIZE: usize = 32; // 256 bits

pub fn generate_vault_key() -> Result<Vec<u8>, CryptoError> {
    let mut key = vec![0u8; VAULT_KEY_SIZE];
    rand::thread_rng().fill_bytes(&mut key);
    Ok(key)
}

pub fn generate_file_key() -> Result<Vec<u8>, CryptoError> {
    let mut key = vec![0u8; FILE_KEY_SIZE];
    rand::thread_rng().fill_bytes(&mut key);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_key_has_correct_size() {
        let key = generate_vault_key().unwrap();
        assert_eq!(key.len(), VAULT_KEY_SIZE);
    }

    #[test]
    fn file_key_has_correct_size() {
        let key = generate_file_key().unwrap();
        assert_eq!(key.len(), FILE_KEY_SIZE);
    }

    #[test]
    fn vault_keys_are_unique() {
        let key1 = generate_vault_key().unwrap();
        let key2 = generate_vault_key().unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn file_keys_are_unique() {
        let key1 = generate_file_key().unwrap();
        let key2 = generate_file_key().unwrap();
        assert_ne!(key1, key2);
    }
}
