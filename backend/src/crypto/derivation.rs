use crate::crypto::error::CryptoError;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Algorithm, Argon2, Version,
};

const KEY_SIZE: usize = 32; // 256 bits

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DerivationParams {
    pub salt: String,
    pub t_cost: u32,
    pub m_cost: u32,
    pub p_cost: u32,
    pub output_len: usize,
}

/// Canonical v1 browser-compatible derivation. The transport salt is decoded first.
pub fn derive_key(password: &str, params: &DerivationParams) -> Result<Vec<u8>, CryptoError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let salt = STANDARD
        .decode(&params.salt)
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid salt: {e}")))?;
    if salt.len() != 16
        || params.t_cost != 2
        || params.m_cost != 19456
        || params.p_cost != 1
        || params.output_len != KEY_SIZE
    {
        return Err(CryptoError::KeyDerivationFailed(
            "Unsupported derivation parameters".into(),
        ));
    }
    derive_with_salt(password, params, &salt)
}

/// Recovery only: reproduces the old Rust salt-text bug for explicitly identified
/// legacy records. Never select this automatically or use it to create new vaults.
pub fn derive_key_legacy_salt_text(
    password: &str,
    params: &DerivationParams,
) -> Result<Vec<u8>, CryptoError> {
    let salt = SaltString::from_b64(&params.salt)
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid legacy salt: {e}")))?;
    derive_with_salt(password, params, salt.as_str().as_bytes())
}

fn derive_with_salt(
    password: &str,
    params: &DerivationParams,
    salt: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        argon2::Params::new(
            params.m_cost,
            params.t_cost,
            params.p_cost,
            Some(params.output_len),
        )
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid params: {e}")))?,
    );
    let mut key = vec![0u8; params.output_len];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Derivation failed: {e}")))?;
    Ok(key)
}

pub fn generate_derivation_params() -> DerivationParams {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use rand::RngCore;
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    DerivationParams {
        salt: STANDARD.encode(salt),
        t_cost: 2,
        m_cost: 19456,
        p_cost: 1,
        output_len: KEY_SIZE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivation_produces_consistent_results() {
        let password = "test-password";
        let params = generate_derivation_params();

        let key1 = derive_key(password, &params).unwrap();
        let key2 = derive_key(password, &params).unwrap();

        assert_eq!(key1, key2);
    }

    #[test]
    fn different_passwords_produce_different_keys() {
        let params = generate_derivation_params();

        let key1 = derive_key("password1", &params).unwrap();
        let key2 = derive_key("password2", &params).unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn different_salts_produce_different_keys() {
        let password = "test-password";
        let params1 = generate_derivation_params();
        let params2 = generate_derivation_params();

        let key1 = derive_key(password, &params1).unwrap();
        let key2 = derive_key(password, &params2).unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn derived_key_has_correct_length() {
        let password = "test-password";
        let params = generate_derivation_params();

        let key = derive_key(password, &params).unwrap();
        assert_eq!(key.len(), KEY_SIZE);
    }
}
