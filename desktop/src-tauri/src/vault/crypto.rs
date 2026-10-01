use super::{
    KDF_ITERATIONS, KDF_MEMORY_KIB, KDF_PARALLELISM, KdfRecord, MASTER_KEY_LEN, NONCE_LEN,
    SALT_LEN, VaultError, WrappedSecret,
};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::{RngCore, rngs::OsRng};
use zeroize::Zeroizing;

pub(super) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(super) fn derive(
    password: &str,
    record: &KdfRecord,
) -> Result<Zeroizing<[u8; MASTER_KEY_LEN]>, VaultError> {
    if record.algorithm != "argon2id-v19"
        || record.m_cost != KDF_MEMORY_KIB
        || record.t_cost != KDF_ITERATIONS
        || record.p_cost != KDF_PARALLELISM
        || record.output_len != MASTER_KEY_LEN
    {
        return Err(VaultError::new(
            "unsupported_kdf",
            "This vault's key derivation parameters are unsupported.",
        ));
    }
    let salt = STANDARD
        .decode(&record.salt)
        .map_err(|_| VaultError::invalid())?;
    if salt.len() != SALT_LEN {
        return Err(VaultError::invalid());
    }
    let params = Params::new(
        KDF_MEMORY_KIB,
        KDF_ITERATIONS,
        KDF_PARALLELISM,
        Some(MASTER_KEY_LEN),
    )
    .map_err(|_| VaultError::invalid())?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut result = Zeroizing::new([0u8; MASTER_KEY_LEN]);
    argon2
        .hash_password_into(password.as_bytes(), &salt, result.as_mut())
        .map_err(|_| VaultError::invalid())?;
    Ok(result)
}

fn aad(id: &str) -> String {
    format!("black-vault:desktop:v1:vault-header:{id}")
}

pub(super) fn seal(
    key: &[u8; MASTER_KEY_LEN],
    id: &str,
    data: &[u8],
) -> Result<WrappedSecret, VaultError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| VaultError::storage())?;
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let context = aad(id);
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: data,
                aad: context.as_bytes(),
            },
        )
        .map_err(|_| VaultError::storage())?;
    Ok(WrappedSecret {
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    })
}

pub(super) fn open(
    key: &[u8; MASTER_KEY_LEN],
    id: &str,
    wrapped: &WrappedSecret,
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let nonce = STANDARD
        .decode(&wrapped.nonce)
        .map_err(|_| VaultError::invalid())?;
    if nonce.len() != NONCE_LEN {
        return Err(VaultError::invalid());
    }
    let ciphertext = STANDARD
        .decode(&wrapped.ciphertext)
        .map_err(|_| VaultError::invalid())?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| VaultError::invalid())?;
    let context = aad(id);
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: context.as_bytes(),
            },
        )
        .map_err(|_| VaultError::invalid())?;
    Ok(Zeroizing::new(plaintext))
}
