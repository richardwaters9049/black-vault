use argon2::{
    password_hash::{
        rand_core::OsRng,
        SaltString,
    },
    Argon2,
    Algorithm,
    Version,
};
use crate::crypto::error::CryptoError;

const KEY_SIZE: usize = 32; // 256 bits

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DerivationParams {
    pub salt: String,
    pub t_cost: u32,
    pub m_cost: u32,
    pub p_cost: u32,
    pub output_len: usize,
}

pub fn derive_key(password: &str, params: &DerivationParams) -> Result<Vec<u8>, CryptoError> {
    let salt = SaltString::from_b64(&params.salt)
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid salt: {}", e)))?;

    let argon2 = Argon2::new(
        Algorithm::Argon2id, 
        Version::V0x13, 
        argon2::Params::new(params.m_cost, params.t_cost, params.p_cost, None)
            .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid params: {}", e)))?
    );

    let mut key = vec![0u8; params.output_len];
    argon2
        .hash_password_into(password.as_bytes(), salt.as_str().as_bytes(), &mut key)
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Derivation failed: {}", e)))?;

    Ok(key)
}

pub fn generate_derivation_params() -> DerivationParams {
    let salt = SaltString::generate(&mut OsRng);
    
    DerivationParams {
        salt: salt.to_string(),
        t_cost: 2,       // Number of iterations
        m_cost: 19456,   // Memory cost in KiB (19 MiB)
        p_cost: 1,       // Parallelism
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
