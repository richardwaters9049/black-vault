//! Authenticated local file envelopes. v1: a per-file random DEK wraps a bounded
//! filename+contents payload. No plaintext is written to Black Vault's storage.
use crate::vault::VaultError;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

#[cfg(unix)]
use std::fs::File;

pub const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_ENVELOPE_BYTES: u64 = 8 * 1024 * 1024;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultFile {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u8,
    vault_id: String,
    file_id: String,
    key_nonce: String,
    wrapped_key: String,
    content_nonce: String,
    content: String,
}
fn invalid() -> VaultError {
    VaultError::file_invalid()
}
fn context(vault_id: &str, file_id: &str, purpose: &str) -> String {
    format!("black-vault:desktop:file:v1:{purpose}:{vault_id}:{file_id}")
}
fn random_nonce() -> [u8; NONCE_LEN] {
    let mut nonce = [0; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    nonce
}
pub fn random_id() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn encrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    data: &[u8],
    aad: &str,
) -> Result<Vec<u8>, VaultError> {
    Aes256Gcm::new_from_slice(key)
        .map_err(|_| invalid())?
        .encrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: data,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| invalid())
}
fn decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    data: &[u8],
    aad: &str,
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    Ok(Zeroizing::new(
        Aes256Gcm::new_from_slice(key)
            .map_err(|_| invalid())?
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: data,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| invalid())?,
    ))
}
fn decode_nonce(value: &str) -> Result<[u8; NONCE_LEN], VaultError> {
    STANDARD
        .decode(value)
        .map_err(|_| invalid())?
        .try_into()
        .map_err(|_| invalid())
}
fn decode(value: &str, max: usize) -> Result<Vec<u8>, VaultError> {
    if value.len() > max * 4 / 3 + 8 {
        return Err(invalid());
    }
    STANDARD.decode(value).map_err(|_| invalid())
}
fn validate_name(name: &str) -> Result<(), VaultError> {
    if name.is_empty()
        || name.len() > 1024
        || name == "."
        || name == ".."
        || name
            .chars()
            .any(|c| c.is_control() || c == '/' || c == '\\')
    {
        return Err(VaultError::file_name());
    }
    Ok(())
}
pub fn seal(
    vault_id: &str,
    file_id: &str,
    master: &[u8; KEY_LEN],
    name: &str,
    content: &[u8],
) -> Result<Vec<u8>, VaultError> {
    if !valid_id(vault_id) || !valid_id(file_id) {
        return Err(invalid());
    }
    validate_name(name)?;
    if content.len() as u64 > MAX_FILE_BYTES {
        return Err(VaultError::file_large());
    }
    let mut dek = Zeroizing::new([0u8; KEY_LEN]);
    OsRng.fill_bytes(dek.as_mut());
    let key_nonce = random_nonce();
    let content_nonce = random_nonce();
    let mut payload = Zeroizing::new(Vec::with_capacity(2 + name.len() + content.len()));
    payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(content);
    let wrapped_key = encrypt(
        master,
        &key_nonce,
        dek.as_ref(),
        &context(vault_id, file_id, "key"),
    )?;
    let ciphertext = encrypt(
        &dek,
        &content_nonce,
        &payload,
        &context(vault_id, file_id, "content"),
    )?;
    serde_json::to_vec(&Envelope {
        version: 1,
        vault_id: vault_id.into(),
        file_id: file_id.into(),
        key_nonce: STANDARD.encode(key_nonce),
        wrapped_key: STANDARD.encode(wrapped_key),
        content_nonce: STANDARD.encode(content_nonce),
        content: STANDARD.encode(ciphertext),
    })
    .map_err(|_| invalid())
}
pub fn open(
    vault_id: &str,
    file_id: &str,
    master: &[u8; KEY_LEN],
    bytes: &[u8],
) -> Result<(VaultFile, Zeroizing<Vec<u8>>), VaultError> {
    if bytes.len() as u64 > MAX_ENVELOPE_BYTES {
        return Err(invalid());
    }
    let record: Envelope = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if record.version != 1
        || record.vault_id != vault_id
        || record.file_id != file_id
        || !valid_id(vault_id)
        || !valid_id(file_id)
    {
        return Err(invalid());
    }
    let wrapped = decode(&record.wrapped_key, KEY_LEN + TAG_LEN)?;
    if wrapped.len() != KEY_LEN + TAG_LEN {
        return Err(invalid());
    }
    let key = decrypt(
        master,
        &decode_nonce(&record.key_nonce)?,
        &wrapped,
        &context(vault_id, file_id, "key"),
    )?;
    let key: &[u8; KEY_LEN] = key.as_slice().try_into().map_err(|_| invalid())?;
    let ciphertext = decode(
        &record.content,
        MAX_FILE_BYTES as usize + 2 + 1024 + TAG_LEN,
    )?;
    let plaintext = decrypt(
        key,
        &decode_nonce(&record.content_nonce)?,
        &ciphertext,
        &context(vault_id, file_id, "content"),
    )?;
    if plaintext.len() < 2 {
        return Err(invalid());
    }
    let name_len = u16::from_be_bytes([plaintext[0], plaintext[1]]) as usize;
    if plaintext.len() < 2 + name_len {
        return Err(invalid());
    }
    let name = std::str::from_utf8(&plaintext[2..2 + name_len]).map_err(|_| invalid())?;
    validate_name(name).map_err(|_| invalid())?;
    let content = Zeroizing::new(plaintext[2 + name_len..].to_vec());
    if content.len() as u64 > MAX_FILE_BYTES {
        return Err(invalid());
    }
    Ok((
        VaultFile {
            id: file_id.into(),
            name: name.into(),
            size_bytes: content.len() as u64,
        },
        content,
    ))
}
pub fn read_regular(path: &Path, max: u64) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let meta = fs::symlink_metadata(path).map_err(|_| VaultError::file_unavailable())?;
    if !meta.file_type().is_file() {
        return Err(VaultError::file_unavailable());
    }
    if meta.len() > max {
        return Err(VaultError::file_large());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| VaultError::file_unavailable())?;
    let opened = file
        .metadata()
        .map_err(|_| VaultError::file_unavailable())?;
    if !opened.is_file() || opened.len() > max {
        return Err(VaultError::file_unavailable());
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| VaultError::file_unavailable())?;
    if bytes.len() as u64 > max {
        return Err(VaultError::file_large());
    }
    Ok(bytes)
}
pub(crate) fn cleanup_file(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        #[cfg(debug_assertions)]
        eprintln!("[black-vault] rollback cleanup failed: {:?}", error.kind());
        #[cfg(not(debug_assertions))]
        let _ = error;
    }
}

pub(crate) fn sync_directory(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        return File::open(dir)?.sync_all();
    }

    #[cfg(windows)]
    {
        // Windows has no supported equivalent of fsync() for a directory.
        // The file itself is flushed before rename; attempting sync_all() on
        // a directory handle returns ERROR_ACCESS_DENIED.
        let _ = dir;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

pub fn persist_new(dir: &Path, id: &str, bytes: &[u8]) -> Result<(), VaultError> {
    if !valid_id(id) || bytes.len() as u64 > MAX_ENVELOPE_BYTES {
        return Err(invalid());
    }
    let temp = dir.join(format!(".{id}.{}.tmp", random_id()));
    let final_path = dir.join(format!("{id}.bvfile"));
    if final_path.exists() {
        return Err(VaultError::storage_error());
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| -> std::io::Result<()> {
        let mut file = options.open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, &final_path)?;
        crate::file_crypto::sync_directory(dir)?;
        Ok(())
    })();
    if result.is_err() {
        cleanup_file(&temp);
        return Err(VaultError::storage_error());
    }
    Ok(())
}
pub fn export_new(path: &Path, bytes: &[u8]) -> Result<(), VaultError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .map_err(|_| VaultError::export_exists())?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        drop(file);
        cleanup_file(path);
        return Err(VaultError::storage_error());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_random_independent_keys() {
        let master = [7u8; 32];
        let vault = random_id();
        let file = random_id();
        let first = seal(&vault, &file, &master, "test.txt", b"hello world").unwrap();
        let second = seal(&vault, &file, &master, "test.txt", b"hello world").unwrap();
        assert_ne!(first, second);
        assert!(!String::from_utf8_lossy(&first).contains("test.txt"));
        assert!(!String::from_utf8_lossy(&first).contains("hello world"));
        let (info, contents) = open(&vault, &file, &master, &first).unwrap();
        assert_eq!(info.name, "test.txt");
        assert_eq!(contents.as_slice(), b"hello world");
    }
    #[test]
    fn rejects_tampering_wrong_keys_and_cross_vault_copy() {
        let master = [1u8; 32];
        let vault = random_id();
        let other = random_id();
        let file = random_id();
        let sealed = seal(&vault, &file, &master, "private.txt", b"private").unwrap();
        assert!(open(&vault, &file, &[2u8; 32], &sealed).is_err());
        assert!(open(&other, &file, &master, &sealed).is_err());
        assert!(open(&vault, &random_id(), &master, &sealed).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&sealed).unwrap();
        let mut data = STANDARD.decode(value["content"].as_str().unwrap()).unwrap();
        data[0] ^= 1;
        value["content"] = STANDARD.encode(data).into();
        assert!(open(&vault, &file, &master, &serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn limits_and_invalid_names() {
        let master = [3u8; 32];
        let vault = random_id();
        let file = random_id();
        assert!(seal(&vault, &file, &master, "../bad", b"hello").is_err());
        assert!(
            seal(
                &vault,
                &file,
                &master,
                "valid",
                &vec![0; MAX_FILE_BYTES as usize + 1]
            )
            .is_err()
        );
        assert!(open(&vault, &file, &master, b"invalid").is_err());
    }
    #[test]
    fn rejects_symlinks_and_export_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("original");
        fs::write(&file, b"source").unwrap();
        assert_eq!(
            read_regular(&file, MAX_FILE_BYTES).unwrap().as_slice(),
            b"source"
        );
        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(read_regular(&link, MAX_FILE_BYTES).is_err());
        }
        assert!(export_new(&file, b"replacement").is_err());
        assert_eq!(fs::read(file).unwrap(), b"source");
    }
}
