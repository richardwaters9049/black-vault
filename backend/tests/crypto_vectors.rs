use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use black_vault_api::crypto::{
    derivation::{derive_key, derive_key_legacy_salt_text, DerivationParams},
    encryption::{decrypt, EncryptedData},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vector {
    password: String,
    params: DerivationParams,
    key: String,
    plaintext: String,
    encrypted: EncryptedData,
}
#[test]
fn shared_vectors_match_derivation_and_encryption() {
    let vectors: Vec<Vector> =
        serde_json::from_str(include_str!("../../tests/fixtures/crypto-v1.json")).unwrap();
    for v in vectors {
        let key = derive_key(&v.password, &v.params).unwrap();
        assert_eq!(STANDARD.encode(&key), v.key);
        let plaintext = STANDARD.decode(v.plaintext).unwrap();
        assert_eq!(decrypt(&v.encrypted, &key).unwrap(), plaintext);
        let nonce = STANDARD.decode(&v.encrypted.nonce).unwrap();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        assert_eq!(
            STANDARD.encode(
                cipher
                    .encrypt(Nonce::from_slice(&nonce), plaintext.as_slice())
                    .unwrap()
            ),
            v.encrypted.ciphertext
        );
    }
}
#[test]
fn legacy_derivation_is_explicit_and_remains_recoverable() {
    let mut params = black_vault_api::crypto::derivation::generate_derivation_params();
    let canonical = derive_key("legacy-test", &params).unwrap();
    params.salt = params.salt.trim_end_matches('=').into();
    assert!(derive_key("legacy-test", &params).is_err());
    let legacy = derive_key_legacy_salt_text("legacy-test", &params).unwrap();
    let mut expected = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(b"legacy-test", params.salt.as_bytes(), &mut expected)
        .unwrap();
    assert_eq!(legacy, expected);
    assert_ne!(legacy, canonical);
}
#[test]
fn rejects_invalid_salt_and_unbounded_parameters() {
    let mut params = black_vault_api::crypto::derivation::generate_derivation_params();
    params.m_cost = u32::MAX;
    assert!(derive_key("test", &params).is_err());
    params.m_cost = 19456;
    params.salt = STANDARD.encode([0u8; 15]);
    assert!(derive_key("test", &params).is_err());
}
